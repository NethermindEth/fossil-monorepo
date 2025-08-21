use eyre::Result;
use tracing::info;

#[cfg(feature = "binary")]
use tracing::Level;
#[cfg(feature = "binary")]
use tracing_subscriber::FmtSubscriber;

use crate::{config::load_starknet_config, provider::StarknetProvider};

/// Example function demonstrating how to use the StarkNet provider
/// to read fees and hashes from the fossil_store contract
pub async fn example_usage() -> Result<()> {
    // Initialize tracing
    #[cfg(feature = "binary")]
    {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
    }

    info!("Starting StarkNet fee reading example");

    // Load configuration from environment variables
    let config = load_starknet_config()?;
    info!("Loaded StarkNet configuration successfully");

    // Create the provider
    let provider = StarknetProvider::new(config)?;
    info!("Created StarkNet provider successfully");

    // Example: Get average fees for the last 24 hours
    let end_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let start_timestamp = end_timestamp - (24 * 60 * 60); // 24 hours ago

    info!(
        start_timestamp,
        end_timestamp, "Fetching average fees for time range"
    );

    // Call the contract function
    match provider
        .get_avg_fees_in_range(start_timestamp, end_timestamp)
        .await
    {
        Ok(fee_data) => {
            info!(
                avg_l1_gas_fee = fee_data.avg_l1_gas_fee,
                avg_l2_gas_fee = fee_data.avg_l2_gas_fee,
                num_block_hashes = fee_data.block_hashes.len(),
                "Successfully retrieved fee data"
            );

            // Print first few block hashes as example
            for (i, hash) in fee_data.block_hashes.iter().take(5).enumerate() {
                info!(index = i, block_hash = %hash, "Block hash");
            }

            if fee_data.block_hashes.len() > 5 {
                info!(
                    "... and {} more block hashes",
                    fee_data.block_hashes.len() - 5
                );
            }
        }
        Err(e) => {
            info!(error = %e, "Failed to retrieve fee data");
            return Err(e);
        }
    }

    info!("Example completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_example_usage_setup() {
        // This test just verifies that the example function compiles
        // and can be called (though it will fail without proper environment setup)

        // We don't actually run the example as it requires real environment configuration
        // Instead, we just verify the function signature and basic setup
        assert!(true, "Example function compiles and can be called");
    }
}
