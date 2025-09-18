use db_access::OffchainProcessorDbConnection;
use dotenv::dotenv;
use server::{create_app, event_monitor::VaultEventMonitor, starknet_provider::StarknetProvider};
use std::{env, error::Error, sync::Arc};
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let offchain_processor_db = Arc::new(OffchainProcessorDbConnection::from_env().await?);

    // Perform db migrations
    offchain_processor_db.migrate().await?;

    let app = create_app(offchain_processor_db.clone()).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    let fmt_layer = fmt::layer()
        .event_format(fmt::format())
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .with_thread_names(true)
        .with_thread_ids(true);

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tracing=info,sqlx=error"));

    Registry::default()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    // Initialize and start event monitor
    let starknet_rpc_url =
        env::var("STARKNET_RPC_URL").unwrap_or_else(|_| "http://localhost:5050".to_string());

    let provider = StarknetProvider::new(&starknet_rpc_url)
        .map_err(|e| format!("Failed to create Starknet provider: {}", e))?;

    let polling_interval_secs = env::var("EVENT_MONITOR_POLLING_INTERVAL")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .map_err(|e| format!("Invalid EVENT_MONITOR_POLLING_INTERVAL: {}", e))?;

    let blocks_per_scan = env::var("EVENT_MONITOR_BLOCKS_PER_SCAN")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<u64>()
        .map_err(|e| format!("Invalid EVENT_MONITOR_BLOCKS_PER_SCAN: {}", e))?;

    let event_monitor = VaultEventMonitor::new(
        provider,
        offchain_processor_db.clone(),
        polling_interval_secs,
        blocks_per_scan,
    );

    info!(
        starknet_rpc_url = %starknet_rpc_url,
        polling_interval_secs = polling_interval_secs,
        blocks_per_scan = blocks_per_scan,
        "Event monitor configured"
    );

    // Start event monitor in background
    let event_monitor_handle = {
        let event_monitor = event_monitor;
        tokio::spawn(async move {
            if let Err(e) = event_monitor.start_monitoring().await {
                error!("Event monitor failed: {:?}", e);
            }
        })
    };

    info!("Server is listening on {}", listener.local_addr()?);
    info!("Event monitor started in background");

    // Start the server
    let server_result = axum::serve(listener, app.into_make_service()).await;

    // If server stops, abort the event monitor
    event_monitor_handle.abort();

    server_result?;
    Ok(())
}
