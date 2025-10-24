// FROST signer node - handles DKG and signing rounds

pub mod config;
pub mod crypto;
pub mod derivation;
pub mod dkg_api;
pub mod dkg_state;
pub mod multi_storage;

use anyhow::Result;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use std::sync::Arc;

pub async fn run(
    server_config: crate::config::ServerConfig,
    node_config: crate::config::NodeConfig,
) -> Result<()> {
    // Load node configuration (network is ignored - signers are chain-agnostic)

    tracing::info!("✅ DKG state initialized");
    tracing::info!(
        "Starting FROST multi-chain signer node {}",
        node_config.node_index
    );
    tracing::info!("Supported curves: secp256k1, Ed25519");

    // Create multi-curve storage
    let multi_storage = Arc::new(multi_storage::MultiCurveStorage::open(
        &node_config.storage_path,
    )?);
    tracing::info!("✅ Multi-curve storage opened");

    // Create shared DKG state
    let dkg_state = Arc::new(dkg_state::DkgState::new());

    // Create unified API (pubkey queries + DKG + FROST signing all in one)
    let api = dkg_api::UnifiedApi {
        config: Arc::new(node_config.clone()),
        storage: multi_storage,
        dkg_state,
    };

    // Single unified API service
    let api_service = OpenApiService::new(api, "FROST Signer Node", "2.0").server(format!(
        "http://{}:{}",
        server_config.host, server_config.port
    ));

    let ui = api_service.rapidoc();
    let spec = api_service.spec_endpoint();

    let app = Route::new()
        .nest("/", api_service)
        .nest("/docs", ui)
        .nest("/spec", spec);

    tracing::info!(
        "🚀 FROST signer node {} listening on {}:{}",
        node_config.node_index,
        server_config.host,
        server_config.port
    );
    tracing::info!("   📊 GET /api/curve/secp256k1/pubkey?passphrase=<uuid>");
    tracing::info!("   🔧 POST /api/dkg/round1|round2|finalize (DKG protocol)");
    tracing::info!("   ✍️  POST /api/frost/round1|round2|aggregate (FROST signing)");
    tracing::info!(
        "   📖 Documentation: http://{}:{}/docs",
        server_config.host,
        server_config.port
    );

    Server::new(TcpListener::bind(format!(
        "{}:{}",
        server_config.host, server_config.port
    )))
    .run(app)
    .await?;

    Ok(())
}
