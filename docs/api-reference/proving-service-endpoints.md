# Proving Service API Reference

## Overview

The Proving Service is an **internal API** used exclusively by the Fossil API to submit proof generation jobs. It provides endpoints for job submission and health monitoring. When jobs are submitted, they are dispatched to an AWS SQS queue for asynchronous processing by proof workers.

### Base URL

```
http://localhost:3001  (Development)
```

Configure via environment variable:
```bash
PROVING_SERVICE_URL=http://localhost:3001
```

### Authentication

Currently, no authentication is required. This is an internal service-to-service API not exposed to external clients.

### Architecture

```
Fossil API → Proving Service HTTP API → AWS SQS Queue → Proof Workers
```

The Proving Service acts as an HTTP gateway to the proof generation pipeline:
1. Receives job requests via HTTP
2. Validates and transforms the request
3. Dispatches jobs to SQS queue
4. Returns immediate confirmation (async processing)

---

## Endpoints

### 1. Health Check

Check if the Proving Service is running and healthy.

#### Request

```http
GET /health
```

#### Response

**Status:** `200 OK`

```json
{
  "status": "healthy",
  "service": "proving-service"
}
```

#### Example

```bash
curl http://localhost:3001/health
```

---

### 2. Submit Proof Job

Submit a new proof generation job to the queue. This is the **primary endpoint** used by Fossil API.

#### Request

```http
POST /api/job
Content-Type: application/json
```

##### Request Body

```json
{
  "job_group_id": "string (required)",
  "twap": {
    "start_timestamp": "i64 (required)",
    "end_timestamp": "i64 (required)"
  },
  "reserve_price": {
    "start_timestamp": "i64 (required)",
    "end_timestamp": "i64 (required)"
  },
  "max_return": {
    "start_timestamp": "i64 (required)",
    "end_timestamp": "i64 (required)"
  },
  "vault_address": "string (optional)",
  "vault_timestamp": "i64 (optional)"
}
```

##### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `job_group_id` | String | Yes | Unique identifier for the job group (used as job_id in the queue) |
| `twap` | TimeRange | Yes | Time range for TWAP (Time-Weighted Average Price) calculation |
| `reserve_price` | TimeRange | Yes | Time range for reserve price calculation |
| `max_return` | TimeRange | Yes | Time range for maximum return calculation |
| `vault_address` | String | No | Ethereum address of the vault contract |
| `vault_timestamp` | i64 | No | Unix timestamp for vault state snapshot |

##### TimeRange Object

```json
{
  "start_timestamp": 1234567890,
  "end_timestamp": 1234567900
}
```

Both timestamps are Unix timestamps (seconds since epoch) as 64-bit integers.

#### Response

##### Success Response

**Status:** `200 OK`

```json
{
  "status": "success",
  "message": "Job dispatched successfully",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

##### Error Response

**Status:** `500 Internal Server Error`

```json
{
  "status": "error",
  "message": "Failed to dispatch job: <error details>",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Examples

##### Basic Request

```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "550e8400-e29b-41d4-a716-446655440000",
    "twap": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "reserve_price": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "max_return": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    }
  }'
```

##### Request with Vault Information

```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "550e8400-e29b-41d4-a716-446655440000",
    "twap": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "reserve_price": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "max_return": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "vault_address": "0x1234567890123456789012345678901234567890",
    "vault_timestamp": 1704153600
  }'
```

---

## Job Format Specification

When a job is submitted via `/api/job`, it's transformed into an internal `Job` enum and serialized to JSON before being sent to the SQS queue.

### Job Enum

```rust
pub enum Job {
    RequestProof(RequestProof),
    ProofGenerated(Box<ProofGenerated>),
}
```

### RequestProof Structure

This is what gets queued when you submit a job:

```rust
pub struct RequestProof {
    pub job_id: String,
    pub job_group_id: Option<String>,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub twap_start_timestamp: Option<i64>,
    pub twap_end_timestamp: Option<i64>,
    pub reserve_price_start_timestamp: Option<i64>,
    pub reserve_price_end_timestamp: Option<i64>,
    pub max_return_start_timestamp: Option<i64>,
    pub max_return_end_timestamp: Option<i64>,
    pub vault_address: Option<String>,
    pub vault_timestamp: Option<i64>,
}
```

### Field Mapping

The HTTP request is transformed into a `RequestProof` job as follows:

| HTTP Request Field | RequestProof Field | Notes |
|-------------------|-------------------|-------|
| `job_group_id` | `job_id` | Used as both job_id and job_group_id |
| `job_group_id` | `job_group_id` | Wrapped in Some() |
| `twap.start_timestamp` | `start_timestamp` | Primary timestamp range |
| `twap.end_timestamp` | `end_timestamp` | Primary timestamp range |
| `twap.start_timestamp` | `twap_start_timestamp` | Wrapped in Some() |
| `twap.end_timestamp` | `twap_end_timestamp` | Wrapped in Some() |
| `reserve_price.start_timestamp` | `reserve_price_start_timestamp` | Wrapped in Some() |
| `reserve_price.end_timestamp` | `reserve_price_end_timestamp` | Wrapped in Some() |
| `max_return.start_timestamp` | `max_return_start_timestamp` | Wrapped in Some() |
| `max_return.end_timestamp` | `max_return_end_timestamp` | Wrapped in Some() |
| `vault_address` | `vault_address` | Optional, passed through |
| `vault_timestamp` | `vault_timestamp` | Optional, passed through |

### SQS Message Format

The job is serialized to JSON using serde and sent to SQS. Example message body:

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000",
  "start_timestamp": 1704067200,
  "end_timestamp": 1704153600,
  "twap_start_timestamp": 1704067200,
  "twap_end_timestamp": 1704153600,
  "reserve_price_start_timestamp": 1704067200,
  "reserve_price_end_timestamp": 1704153600,
  "max_return_start_timestamp": 1704067200,
  "max_return_end_timestamp": 1704153600,
  "vault_address": "0x1234567890123456789012345678901234567890",
  "vault_timestamp": 1704153600
}
```

The message is wrapped in the `Job::RequestProof` variant using serde's untagged enum serialization.

---

## Integration Guide

### How Fossil API Calls the Proving Service

The Fossil API integrates with the Proving Service through the `call_proving_service` function located in:
```
fossil-api/crates/server/src/handlers/get_pricing_data.rs
```

#### Integration Flow

1. **Fossil API receives pricing request** from external client
2. **Creates job record** in PostgreSQL with `Pending` status
3. **Calls Proving Service** via HTTP POST to `/api/job`
4. **Proving Service queues job** to AWS SQS
5. **Returns success** to Fossil API immediately
6. **Proof workers** pick up job from queue asynchronously
7. **On completion**, workers trigger on-chain confirmation
8. **Fossil API listens** for `FossilCallbackSuccess` event
9. **Updates job status** to `Completed`

#### Code Example: Fossil API Integration

```rust
async fn call_proving_service(
    job_id: &str,
    payload: &PitchLakeJobRequest,
) -> Result<serde_json::Value, eyre::Error> {
    // Get proving service URL from environment
    let proving_service_url = env::var("PROVING_SERVICE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

    // Build payload for proving service
    let vault_timestamp = get_current_timestamp()?;
    let api_payload = json!({
        "job_group_id": job_id,
        "twap": {
            "start_timestamp": payload.params.twap.0,
            "end_timestamp": payload.params.twap.1
        },
        "reserve_price": {
            "start_timestamp": payload.params.reserve_price.0,
            "end_timestamp": payload.params.reserve_price.1
        },
        "max_return": {
            "start_timestamp": payload.params.max_return.0,
            "end_timestamp": payload.params.max_return.1
        },
        "vault_address": payload.vault_address,
        "vault_timestamp": vault_timestamp
    });

    // Send request to proving service
    let response = Client::new()
        .post(format!("{}/api/job", proving_service_url))
        .json(&api_payload)
        .send()
        .await?;

    // Parse response
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre!("Proving service returned error: {}", error_text));
    }

    let result = response.json::<serde_json::Value>().await?;
    Ok(result)
}
```

#### Environment Configuration

Fossil API requires the following environment variable:

```bash
# .env file
PROVING_SERVICE_URL=http://localhost:3001
```

For production:
```bash
PROVING_SERVICE_URL=http://proving-service:3001
```

#### Error Handling

The Fossil API handles proving service errors by:
1. Catching any errors from the HTTP call
2. Logging the error with full context
3. Updating the job status to `Failed` in PostgreSQL
4. Allowing clients to retry by re-submitting the same job

---

## Error Responses

### Error Response Format

All errors from the Proving Service follow this structure:

```json
{
  "status": "error",
  "message": "string describing the error",
  "job_group_id": "original job_group_id from request"
}
```

### Error Scenarios

#### 1. Queue Send Failure

**Status:** `500 Internal Server Error`

Occurs when the SQS queue is unavailable or rejects the message.

```json
{
  "status": "error",
  "message": "Failed to dispatch job: Failed to send message: <SQS error details>",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Common Causes:**
- SQS queue not configured correctly
- AWS credentials invalid or expired
- Network connectivity issues
- LocalStack not running (development)

**Resolution:**
- Check `SQS_QUEUE_URL` environment variable
- Verify AWS credentials
- Ensure LocalStack is running: `make dev-services`
- Check network connectivity to AWS

#### 2. Invalid JSON

**Status:** `400 Bad Request` (handled by Axum)

Occurs when the request body is not valid JSON or doesn't match the expected schema.

```json
{
  "error": "Failed to deserialize JSON body"
}
```

**Common Causes:**
- Missing required fields
- Invalid field types (e.g., string instead of integer)
- Malformed JSON syntax

#### 3. Serialization Error

**Status:** `500 Internal Server Error`

Occurs when the job cannot be serialized to JSON for the queue.

```json
{
  "status": "error",
  "message": "Failed to dispatch job: serialization error",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Common Causes:**
- Invalid data in job structure (rare, usually a bug)

### HTTP Status Codes

| Status Code | Meaning | When It Occurs |
|------------|---------|----------------|
| `200 OK` | Success | Job successfully queued |
| `400 Bad Request` | Invalid request | Malformed JSON or missing fields |
| `500 Internal Server Error` | Server error | Queue failure, serialization error |

---

## Examples

### Complete Integration Example

Here's a complete example showing how an external system would interact with both Fossil API and the Proving Service:

#### Step 1: Submit Job to Fossil API

```bash
curl -X POST http://localhost:3000/api/pricing \
  -H "Content-Type: application/json" \
  -d '{
    "program_id": "pitchlake-v1",
    "params": {
      "twap": [1704067200, 1704153600],
      "reserve_price": [1704067200, 1704153600],
      "max_return": [1704067200, 1704153600]
    },
    "vault_address": "0x1234567890123456789012345678901234567890"
  }'
```

**Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "New job request registered and processing initiated.",
  "status": "Pending",
  "vault_address": "0x1234567890123456789012345678901234567890",
  "expected_timestamp": 1704153600
}
```

#### Step 2: Fossil API Calls Proving Service (Internal)

This happens automatically. Fossil API transforms the request and calls:

```bash
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "550e8400-e29b-41d4-a716-446655440000",
    "twap": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "reserve_price": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "max_return": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "vault_address": "0x1234567890123456789012345678901234567890",
    "vault_timestamp": 1704153600
  }'
```

**Response:**
```json
{
  "status": "success",
  "message": "Job dispatched successfully",
  "job_group_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Step 3: Check Job Status

```bash
curl http://localhost:3000/api/job/550e8400-e29b-41d4-a716-446655440000/status
```

**Response (while processing):**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending"
}
```

**Response (after completion):**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "vault_address": "0x1234567890123456789012345678901234567890"
}
```

### Testing with LocalStack (Development)

For local development, the Proving Service uses LocalStack to simulate AWS SQS:

```bash
# 1. Start LocalStack and PostgreSQL
make dev-services

# 2. Start Proving Service
cd proving-service
cargo run

# 3. In another terminal, submit a test job
curl -X POST http://localhost:3001/api/job \
  -H "Content-Type: application/json" \
  -d '{
    "job_group_id": "test-job-123",
    "twap": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "reserve_price": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    },
    "max_return": {
      "start_timestamp": 1704067200,
      "end_timestamp": 1704153600
    }
  }'

# 4. Check SQS queue in LocalStack
aws --endpoint-url=http://localhost:4566 sqs receive-message \
  --queue-url http://localhost:4566/000000000000/fossilQueue
```

---

## Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `SQS_QUEUE_URL` | AWS SQS queue URL for job messages | `http://localhost:4566/000000000000/fossilQueue` | Yes |
| `AWS_REGION` | AWS region for SQS | `us-east-1` | No |
| `AWS_ACCESS_KEY_ID` | AWS credentials | - | Yes (production) |
| `AWS_SECRET_ACCESS_KEY` | AWS credentials | - | Yes (production) |

### Development Configuration

For local development with LocalStack:

```bash
# .env
SQS_QUEUE_URL=http://localhost:4566/000000000000/fossilQueue
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
```

### Production Configuration

For production with AWS:

```bash
# .env
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/123456789012/fossilQueue
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=<your-access-key>
AWS_SECRET_ACCESS_KEY=<your-secret-key>
```

---

## Monitoring and Debugging

### Logging

The Proving Service uses `tracing` for structured logging. Logs include:

- Job receipt: `"Received job request for group: {job_group_id}"`
- Serialization debug: `"Serializing RequestProof: vault_address={:?}, vault_timestamp={:?}"`
- Queue send: `"Sending message to queue: {message_body}"`
- Success: `"Successfully dispatched job with ID: {job_id}"`
- Errors: `"Failed to dispatch job with ID: {job_id}. Error: {error}"`

### Health Monitoring

Monitor service health:

```bash
# Simple health check
curl http://localhost:3001/health

# Monitor with watch
watch -n 5 'curl -s http://localhost:3001/health | jq'
```

### SQS Queue Monitoring

Check queue depth and messages:

```bash
# LocalStack
aws --endpoint-url=http://localhost:4566 sqs get-queue-attributes \
  --queue-url http://localhost:4566/000000000000/fossilQueue \
  --attribute-names All

# Production
aws sqs get-queue-attributes \
  --queue-url https://sqs.us-east-1.amazonaws.com/123456789012/fossilQueue \
  --attribute-names All
```

### Common Issues

#### Service won't start
- Check if port 3001 is already in use: `lsof -i :3001`
- Verify `.env` file exists and is properly configured
- Check AWS credentials are valid

#### Jobs not appearing in queue
- Verify `SQS_QUEUE_URL` is correct
- Check LocalStack is running: `docker ps | grep localstack`
- Review service logs for serialization errors
- Test queue connectivity: `aws --endpoint-url=http://localhost:4566 sqs list-queues`

#### 500 errors on job submission
- Check service logs for detailed error messages
- Verify SQS queue exists and is accessible
- Ensure AWS credentials have permissions to send messages
- Check network connectivity to AWS/LocalStack

---

## Related Documentation

- [Fossil API Documentation](../api-reference/fossil-api-endpoints.md)
- [Architecture Overview](../architecture/proving-service.md)
- [SQS Queue Configuration](../guides/queue-setup.md)
- [Development Setup](../getting-started/setup.md)

---

## Changelog

### Version 1.0.0 (Current)

- Initial API implementation
- Single combined job submission (TWAP, reserve price, max return)
- Vault address and timestamp support
- AWS SQS integration
- LocalStack support for development
- Health check endpoint
