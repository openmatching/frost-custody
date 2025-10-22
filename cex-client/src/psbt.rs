use anyhow::{Context, Result};
use bitcoin::psbt::{Input, Psbt};
use bitcoin::{
    absolute, transaction, Address, Amount, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Witness,
};

/// UTXO information needed to build PSBT
#[derive(Debug, Clone)]
pub struct Utxo {
    pub txid: bitcoin::Txid,
    pub vout: u32,
    pub amount: Amount,
    pub address: Address,
    pub passphrase: String,
}

/// Build a consolidation PSBT from multiple UTXOs
///
/// # Arguments
/// * `utxos` - List of UTXOs to consolidate
/// * `destination` - Cold wallet address
/// * `fee_amount` - Total fee to pay
///
/// # Returns
/// Unsigned PSBT ready to be signed by signer nodes
pub fn build_consolidation_psbt(
    utxos: Vec<Utxo>,
    destination: Address,
    fee_amount: Amount,
) -> Result<(Psbt, Vec<String>)> {
    if utxos.is_empty() {
        anyhow::bail!("Cannot build PSBT with zero UTXOs");
    }

    // Calculate total input amount
    let total_input: u64 = utxos.iter().map(|u| u.amount.to_sat()).sum();
    let output_amount = total_input
        .checked_sub(fee_amount.to_sat())
        .context("Fee exceeds total input amount")?;

    // Build unsigned transaction
    let tx = Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: utxos
            .iter()
            .map(|utxo| TxIn {
                previous_output: OutPoint {
                    txid: utxo.txid,
                    vout: utxo.vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(output_amount),
            script_pubkey: destination.script_pubkey(),
        }],
    };

    // Create PSBT from unsigned transaction
    let mut psbt = Psbt::from_unsigned_tx(tx)?;

    // Add witness UTXO and witness script for each input
    for (i, utxo) in utxos.iter().enumerate() {
        psbt.inputs[i] = Input {
            witness_utxo: Some(TxOut {
                value: utxo.amount,
                script_pubkey: utxo.address.script_pubkey(),
            }),
            // For multisig, witness_script will be needed
            // CEX must derive the 3 pubkeys and create the script
            // (we'll add this in the example)
            ..Default::default()
        };
    }

    // Extract passphrases in input order
    let passphrases: Vec<String> = utxos.iter().map(|u| u.passphrase.clone()).collect();

    Ok((psbt, passphrases))
}

/// Add witness scripts to PSBT inputs for multisig
///
/// This must be called before signing. It derives the 3 pubkeys for each input
/// and adds the multisig witness script.
pub fn add_witness_scripts(
    psbt: &mut Psbt,
    xpubs: &[bitcoin::bip32::Xpub],
    passphrases: &[String],
) -> Result<()> {
    use bitcoin::blockdata::opcodes::all::OP_CHECKMULTISIG;
    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::PublicKey;

    if passphrases.len() != psbt.inputs.len() {
        anyhow::bail!("Passphrases count must match PSBT inputs count");
    }

    let secp = Secp256k1::new();

    for (i, passphrase) in passphrases.iter().enumerate() {
        // Convert passphrase to 9-level derivation path
        let path = crate::passphrase_to_derivation_path(passphrase);

        // Derive 3 pubkeys
        let mut pubkeys = Vec::new();
        for xpub in xpubs {
            let child_xpub = xpub
                .derive_pub(&secp, &path)
                .context("Failed to derive child pubkey")?;
            pubkeys.push(PublicKey::new(child_xpub.public_key));
        }

        // Sort for sortedmulti
        pubkeys.sort();

        // Create 2-of-3 multisig witness script
        let witness_script = bitcoin::blockdata::script::Builder::new()
            .push_int(2)
            .push_key(&pubkeys[0])
            .push_key(&pubkeys[1])
            .push_key(&pubkeys[2])
            .push_int(3)
            .push_opcode(OP_CHECKMULTISIG)
            .into_script();

        // Add to PSBT input
        psbt.inputs[i].witness_script = Some(witness_script);
    }

    Ok(())
}

/// Serialize PSBT to base64 for API calls
pub fn psbt_to_base64(psbt: &Psbt) -> String {
    use base64::prelude::*;
    BASE64_STANDARD.encode(psbt.serialize())
}

/// Deserialize PSBT from base64
pub fn psbt_from_base64(base64_str: &str) -> Result<Psbt> {
    use base64::prelude::*;
    let bytes = BASE64_STANDARD
        .decode(base64_str)
        .context("Failed to decode base64 PSBT")?;
    Psbt::deserialize(&bytes).context("Failed to deserialize PSBT")
}
