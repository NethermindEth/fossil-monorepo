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
    "job_group_id": "job_123",
    "twap": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "reserve_price": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "max_return": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    }
}
```

The `job_group_id` field is required and groups all three proofs together. Each proof type (twap, reserve_price, max_return) requires its own time range.

### Response Format

#### Success Response

```json
{
    "status": "success",
    "message": "All jobs dispatched successfully",
    "job_group_id": "job_123"
}
```

#### Error Response

```json
{
    "status": "error",
    "message": "TWAP job failed: error1, Reserve Price job failed: error2",
    "job_group_id": "job_123"
}
```

### Example Usage with curl

```bash
curl -X POST http://127.0.0.1:3000/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "job_123",
    "twap": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "reserve_price": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    },
    "max_return": {
        "start_timestamp": 1234567890,
        "end_timestamp": 1234567891
    }
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
            "job_group_id": "job_123",
            "twap": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            },
            "reserve_price": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            },
            "max_return": {
                "start_timestamp": 1234567890,
                "end_timestamp": 1234567891
            }
        }))
        .send()
        .await?;
    
    let result = response.json::<serde_json::Value>().await?;
    println!("Response: {:?}", result);
    
    Ok(())
}
```
