# Prover service

A service that processes jobs through AWS SQS and exposes an HTTP API for job submission.

## HTTP API

The service exposes a single HTTP endpoint for submitting jobs:

### Endpoint

```bash
POST http://127.0.0.1:3000/api/job
```

### Request Format

Send a POST request with a JSON body in the following format:

```json
{
    "type": "twap",  // One of: "twap", "reserve-price", "max-return"
    "start_timestamp": 1234567890,
    "end_timestamp": 1234567891,
    "job_group_id": "job_123"  // Optional: Groups related proofs together
}
```

The `job_group_id` field is optional but recommended when submitting multiple proofs that are part of the same job. This helps track and manage related proofs together.

### Response Format

#### Success Response

```json
{
    "status": "success",
    "message": "Job dispatched successfully",
    "job_id": "twap",
    "job_group_id": "job_123"  // Echoes back the job_group_id if provided
}
```

#### Error Response

```json
{
    "status": "error",
    "message": "Error message here",
    "job_id": "twap",
    "job_group_id": "job_123"  // Echoes back the job_group_id if provided
}
```

### Example Usage with curl

```bash
curl -X POST http://127.0.0.1:3000/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "type": "twap",
    "start_timestamp": 1234567890,
    "end_timestamp": 1234567891,
    "job_group_id": "job_123"
  }'
```

### Example Usage with Rust

```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let response = client
        .post("http://127.0.0.1:3000/api/job")
        .json(&json!({
            "type": "twap",
            "start_timestamp": 1234567890,
            "end_timestamp": 1234567891,
            "job_group_id": "job_123"
        }))
        .send()
        .await?;
    
    let result = response.json::<serde_json::Value>().await?;
    println!("Response: {:?}", result);
    
    Ok(())
}
```
