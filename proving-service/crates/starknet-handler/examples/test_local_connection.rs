use eyre::Result;
use starknet::providers::Provider;
use starknet_handler::provider::{StarkNetConfig, StarknetProvider};

/// Simple test to verify connection to local StarkNet node
/// Usage: cargo run --example test_local_connection
#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing StarkNet Handler with Local RPC");
    println!("==========================================");

    // Configuration - load from environment or use defaults
    let rpc_url =
        std::env::var("STARKNET_RPC_URL").unwrap_or_else(|_| "http://localhost:5050".to_string());
    let contract_address = std::env::var("FOSSIL_STORE_ADDRESS").unwrap_or_else(|_| {
        "0x027a6e498bfad98a5145ac507f90fac6268cce6c0373c6eecd38de46be655b55".to_string()
    });
    let start_timestamp = 1755457200u64;
    let end_timestamp = 1755464400u64;

    println!("📋 Test Configuration:");
    println!("   RPC URL: {}", rpc_url);
    println!("   Contract: {}", contract_address);
    println!("   Start Time: {}", start_timestamp);
    println!("   End Time: {}", end_timestamp);

    // Create configuration
    let config = StarkNetConfig::new(rpc_url.to_string(), contract_address.to_string())
        .with_retry_config(3, 100, 1000);

    println!("\n🔧 Creating StarkNet provider...");
    let provider = StarknetProvider::new(config)?;

    println!("✅ Provider created successfully");
    println!("   Provider RPC URL: {}", provider.rpc_url());
    println!("   Contract Address: {}", provider.fossil_store_address());

    // Test basic connectivity
    println!("\n📡 Testing RPC connectivity...");
    let client = provider.provider();

    match client.chain_id().await {
        Ok(chain_id) => {
            println!("✅ Successfully connected to StarkNet node");
            println!("   Chain ID: {}", chain_id);
        }
        Err(e) => {
            println!("❌ Failed to connect to StarkNet node: {}", e);
            println!("ℹ️  Make sure a StarkNet node is running on {}", rpc_url);
            return Ok(());
        }
    }

    // Test contract call
    println!("\n📜 Testing contract call: get_avg_fees_in_range");
    println!("   This will call the fossil_store contract...");

    match provider
        .get_avg_fees_in_range(start_timestamp, end_timestamp)
        .await
    {
        Ok(fee_data) => {
            println!("🎉 Successfully retrieved fee data!");
            println!("   First Timestamp: {}", fee_data.first_timestamp);
            println!("   Last Timestamp: {}", fee_data.last_timestamp);
            println!("   Number of Fees: {}", fee_data.fees.len());

            // Show first few fees
            if !fee_data.fees.is_empty() {
                println!("   Sample Fee Values:");
                for (i, fee) in fee_data.fees.iter().take(3).enumerate() {
                    println!("     {}: {:#x}", i + 1, fee);
                }
                if fee_data.fees.len() > 3 {
                    println!("     ... and {} more", fee_data.fees.len() - 3);
                }
            }
        }
        Err(e) => {
            println!("❌ Contract call failed: {}", e);
            println!("ℹ️  Possible reasons:");
            println!("   - Contract not deployed at specified address");
            println!("   - Contract missing get_avg_fees_in_range function");
            println!("   - Network connectivity issues");
            println!("   - Invalid timestamp range");
        }
    }

    println!("\n✨ Test completed!");
    Ok(())
}
