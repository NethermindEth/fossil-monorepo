# DB Access Crate

This crate provides database access functionality for:

- API key management
- Job request handling

## Example Usage

```rust
use db_access::{OffchainProcessorDbConnection, models::JobStatus};
use db_access::auth::{add_api_key, validate_api_key};
use db_access::queries::{create_job_request, get_job_request, update_job_status};
use std::sync::Arc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize the database connection
    let db = Arc::new(OffchainProcessorDbConnection::from_env().await?);
    
    // Run migrations if needed
    db.migrate().await?;

    // API Key Management
    let api_key = "my-api-key-123";
    let name = "Example API Key";
    
    // Add a new API key
    add_api_key(db.clone(), api_key.to_string(), name.to_string()).await?;
    
    // Validate an API key
    match validate_api_key(db.clone(), api_key).await {
        Ok(_) => println!("API key is valid"),
        Err(_) => println!("API key is invalid"),
    }
    
    // Job Request Management
    let job_id = "job-123";
    
    // Create a new job request
    create_job_request(db.clone(), job_id, JobStatus::Pending).await?;
    
    // Get job request status
    if let Some(job) = get_job_request(db.clone(), job_id).await? {
        println!("Job status: {}", job.status);
    }
    
    // Update job status
    let result = serde_json::json!({
        "success": true,
        "data": {
            "timestamp": "2023-10-24T12:34:56Z"
        }
    });
    
    update_job_status(db.clone(), job_id, JobStatus::Completed, Some(result)).await?;
    
    Ok(())
}
```
