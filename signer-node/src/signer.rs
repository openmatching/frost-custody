use anyhow::{Context, Result};
use base64::prelude::*;
use bitcoin::hashes::Hash;
use bitcoin::psbt::Psbt;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{Prevouts, SighashCache};

use crate::config::SignerNode;

pub fn sign_psbt(
    config: &SignerNode,
    psbt_str: &str,
    passphrases: &[String],
) -> Result<(String, usize)> {
    // Decode PSBT from base64
    let psbt_bytes = BASE64_STANDARD
        .decode(psbt_str)
        .context("Failed to decode base64 PSBT")?;
    let mut psbt = Psbt::deserialize(&psbt_bytes).context("Failed to deserialize PSBT")?;

    let secp = Secp256k1::new();
    let mut signed_count = 0;

    // Sign each input with the provided passphrase
    for (idx, passphrase) in passphrases.iter().enumerate() {
        if idx >= psbt.inputs.len() {
            tracing::warn!(
                "Passphrase index {} exceeds PSBT inputs count {}",
                idx,
                psbt.inputs.len()
            );
            break;
        }

        // Derive private key from passphrase
        let privkey = config
            .derive_privkey(passphrase)
            .context(format!("Failed to derive privkey for passphrase"))?;

        // Try to sign this input
        match sign_input(&secp, &mut psbt, idx, &privkey) {
            Ok(_) => {
                signed_count += 1;
                tracing::debug!("Signed input {} with passphrase", idx);
            }
            Err(e) => {
                tracing::warn!("Failed to sign input {}: {}", idx, e);
            }
        }
    }

    // Encode PSBT back to base64
    let signed_psbt = BASE64_STANDARD.encode(psbt.serialize());

    Ok((signed_psbt, signed_count))
}

fn sign_input(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    psbt: &mut Psbt,
    input_idx: usize,
    privkey: &bitcoin::PrivateKey,
) -> Result<()> {
    let input = &psbt.inputs[input_idx];

    // For P2WSH (multisig), we need witness_script
    let witness_script = input
        .witness_script
        .as_ref()
        .context("Missing witness_script in PSBT input")?;

    // Get the previous output for this input
    let prev_output = input
        .witness_utxo
        .as_ref()
        .context("Missing witness_utxo in PSBT input")?;

    // Create sighash
    let mut sighash_cache = SighashCache::new(&psbt.unsigned_tx);

    let prevout_vec = vec![prev_output.clone()]; // Simplified: assume single prevout
    let _prevouts = Prevouts::All(&prevout_vec);

    let sighash = sighash_cache
        .p2wsh_signature_hash(
            input_idx,
            witness_script,
            prev_output.value,
            bitcoin::sighash::EcdsaSighashType::All,
        )
        .context("Failed to compute sighash")?;

    // Sign
    let message = Message::from_digest(*sighash.as_byte_array());
    let signature = secp.sign_ecdsa(&message, &privkey.inner);

    // Create Bitcoin signature (signature + sighash type)
    let mut sig_bytes = signature.serialize_der().to_vec();
    sig_bytes.push(bitcoin::sighash::EcdsaSighashType::All.to_u32() as u8);

    // Add to partial_sigs
    let pubkey = privkey.public_key(secp);
    psbt.inputs[input_idx]
        .partial_sigs
        .insert(pubkey, bitcoin::ecdsa::Signature::from_slice(&sig_bytes)?);

    Ok(())
}

pub fn derive_multisig_address(config: &SignerNode, passphrase: &str) -> Result<String> {
    let secp = Secp256k1::new();

    // Convert passphrase to 9-level derivation path (full 256-bit space, standard BIP32)
    let path = SignerNode::passphrase_to_derivation_path(passphrase);

    // Derive child pubkeys from all 3 xpubs (standard BIP32!)
    let mut pubkeys = Vec::new();
    for xpub in &config.all_xpubs {
        let child_xpub = xpub
            .derive_pub(&secp, &path)
            .context("Failed to derive child pubkey")?;

        pubkeys.push(bitcoin::PublicKey::new(child_xpub.public_key));
    }

    // Sort pubkeys for sortedmulti
    pubkeys.sort();

    // Create 2-of-3 multisig witness script
    let script = bitcoin::blockdata::script::Builder::new()
        .push_int(2)
        .push_key(&pubkeys[0])
        .push_key(&pubkeys[1])
        .push_key(&pubkeys[2])
        .push_int(3)
        .push_opcode(bitcoin::blockdata::opcodes::all::OP_CHECKMULTISIG)
        .into_script();

    // Create P2WSH address
    let address = bitcoin::Address::p2wsh(&script, config.network);

    Ok(address.to_string())
}
