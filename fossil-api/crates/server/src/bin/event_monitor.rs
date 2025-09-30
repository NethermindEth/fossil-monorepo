use std::{env, sync::Arc};

use db_access::OffchainProcessorDbConnection;
use dotenv::dotenv;
use eyre::Result;
use server::{event_monitor::VaultEventMonitor, starknet_provider::StarknetProvider};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "event_monitor=debug,server=debug,db_access=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Fossil Event Monitor");

    // Initialize database connection
    let offchain_processor_db = Arc::new(OffchainProcessorDbConnection::from_env().await?);

    // Initialize Starknet provider
    let starknet_rpc_url =
        env::var("STARKNET_RPC_URL").unwrap_or_else(|_| "http://localhost:5050".to_string());

    let provider = StarknetProvider::new(&starknet_rpc_url)
        .map_err(|e| eyre::eyre!("Failed to create Starknet provider: {}", e))?;

    // Configure event monitor
    let polling_interval_secs = env::var("EVENT_MONITOR_POLLING_INTERVAL")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .map_err(|e| eyre::eyre!("Invalid EVENT_MONITOR_POLLING_INTERVAL: {}", e))?;

    let blocks_per_scan = env::var("EVENT_MONITOR_BLOCKS_PER_SCAN")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<u64>()
        .map_err(|e| eyre::eyre!("Invalid EVENT_MONITOR_BLOCKS_PER_SCAN: {}", e))?;

    let event_monitor = VaultEventMonitor::new(
        provider,
        offchain_processor_db,
        polling_interval_secs,
        blocks_per_scan,
    );

    info!(
        starknet_rpc_url = %starknet_rpc_url,
        polling_interval_secs = polling_interval_secs,
        blocks_per_scan = blocks_per_scan,
        "Event monitor configured"
    );

    // Start monitoring
    if let Err(e) = event_monitor.start_monitoring().await {
        error!("Event monitor failed: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
