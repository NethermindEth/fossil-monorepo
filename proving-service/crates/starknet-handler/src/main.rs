use eyre::Result;
use starknet_handler::example::example_usage;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenv::dotenv().ok();

    // Run the example
    example_usage().await
}
