# Fossil API Reference

Complete API documentation for the Fossil API - a service that processes pricing data requests for decentralized finance applications.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Base URL](#base-url)
- [Response Formats](#response-formats)
- [Endpoints](#endpoints)
  - [Health Check](#health-check)
  - [Create API Key](#create-api-key)
  - [Submit Pricing Data Request](#submit-pricing-data-request)
  - [Get Job Status](#get-job-status)
  - [Get Job Result](#get-job-result)
  - [Get Batch Job Status](#get-batch-job-status)
  - [Webhook Callback](#webhook-callback)
- [Error Codes](#error-codes)
- [Rate Limiting](#rate-limiting)
- [Common Use Cases](#common-use-cases)
- [Next Steps](#next-steps)

---

## Overview

The Fossil API provides endpoints for submitting pricing data requests and retrieving job results. The API follows RESTful principles and returns JSON responses.

### Key Features

- **Asynchronous Processing**: Submit jobs and retrieve results when ready
- **Batch Operations**: Query multiple jobs in a single request
- **Event-driven Architecture**: Automatic on-chain confirmation tracking
- **Idempotent Requests**: Duplicate requests are handled gracefully

### API Versioning

The current API version is `v1`. The API is currently unversioned in the URL path but follows semantic versioning principles.

### Base URL

**Development/Local:**
```
http://localhost:3000
```

**Production:**
```
https://api.fossil.your-domain.com
```

### Response Formats

All responses are returned in JSON format with appropriate HTTP status codes.

**Successful Response:**
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending",
  "message": "Job created successfully"
}
```

**Error Response:**
```json
{
  "error": "Authentication failed: No API key provided in headers"
}
```

---

## Authentication

Most endpoints require API key authentication. Include your API key in the request headers.

### Header Format

```
X-API-Key: your-api-key-here
```

### Obtaining an API Key

Use the [Create API Key](#create-api-key) endpoint to generate a new API key. Store it securely - you cannot retrieve it later.

### Authentication Errors

| Error | Status Code | Description |
|-------|-------------|-------------|
| No API key provided in headers | 401 | Missing `X-API-Key` header |
| Invalid API key format | 401 | Malformed API key |
| API key not found | 401 | Invalid or revoked API key |
| Database error occurred while validating API key | 401 | Internal authentication error |

### Example

```bash
curl -X POST https://api.fossil.your-domain.com/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{"program_id": "PITCHLAKE_V1", ...}'
```

---

## Endpoints

### Health Check

Check if the API service is running.

**Endpoint:** `GET /health`

**Authentication:** Not required

**Description:** Returns a simple health status. This is primarily a liveness check and does not verify database connectivity or other dependencies.

#### Request

```bash
curl -X GET http://localhost:3000/health
```

#### Response

**Status Code:** `200 OK`

**Body:**
```
OK
```

---

### Create API Key

Generate a new API key for authentication.

**Endpoint:** `POST /api_key`

**Authentication:** Not required

**Description:** Creates a new API key that can be used to authenticate requests to protected endpoints.

#### Request

**Headers:**
```
Content-Type: application/json
```

**Body:**
```json
{
  "name": "my_api_key"
}
```

**Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | string | Yes | A descriptive name for the API key |

#### Response

**Status Code:** `200 OK`

**Body:**
```json
{
  "api_key": "724a0c8d-9fea-4c7c-97a0-1aea894a283e"
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| api_key | string | UUID v4 formatted API key |

#### Example

```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "production_key"}'
```

#### Error Responses

**Status Code:** `500 Internal Server Error`

```json
{
  "error": "Failed to store API key"
}
```

**Important:** Save the API key securely. You cannot retrieve it later.

---

### Submit Pricing Data Request

Submit a request to process pricing data for a specific vault and time ranges.

**Endpoint:** `POST /pricing_data`

**Authentication:** Required

**Description:** This is the main endpoint for submitting pricing data calculation requests. The API processes three types of calculations: TWAP (Time-Weighted Average Price), Maximum Return, and Reserve Price, each over specified time ranges.

#### Request

**Headers:**
```
Content-Type: application/json
X-API-Key: your-api-key-here
```

**Body:**
```json
{
  "program_id": "PITCHLAKE_V1",
  "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
  "params": {
    "twap": [1672531200, 1672574400],
    "max_return": [1672531200, 1672574400],
    "reserve_price": [1672531200, 1672574400]
  }
}
```

**Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| program_id | string | Yes | Program identifier (e.g., "PITCHLAKE_V1"). Cannot be empty. |
| vault_address | string | Yes | StarkNet vault contract address in hex format |
| params | object | Yes | Calculation parameters |
| params.twap | [int, int] | Yes | Unix timestamp range for TWAP calculation [start, end] |
| params.max_return | [int, int] | Yes | Unix timestamp range for max return calculation [start, end] |
| params.reserve_price | [int, int] | Yes | Unix timestamp range for reserve price calculation [start, end] |

**Validation Rules:**
- `program_id` must not be empty
- For each time range: `start_timestamp < end_timestamp`
- All timestamps must be valid Unix timestamps (seconds since epoch)

#### Response

The API returns different responses based on job status:

##### New Job Created

**Status Code:** `201 Created`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "New job request registered and processing initiated.",
  "status": "Pending",
  "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
  "expected_timestamp": 1672574400
}
```

##### Job Already Pending

**Status Code:** `409 Conflict`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Job is already pending. Use the status endpoint to monitor progress."
}
```

##### Job Already Completed

**Status Code:** `200 OK`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Job has already been completed. No further processing required."
}
```

##### Failed Job Being Reprocessed

**Status Code:** `200 OK`

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Previous job request failed. Reprocessing initiated."
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| job_id | string | Unique job identifier (UUID v4 or deterministic ID) |
| message | string | Human-readable status message |
| status | string | Job status: "Pending", "Completed", or "Failed" |
| vault_address | string | Vault address from the request |
| expected_timestamp | integer | Expected completion timestamp (Unix) |

#### Example

```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
```

#### Error Responses

**Status Code:** `400 Bad Request` - Invalid Parameters

```json
{
  "job_id": "",
  "message": "Program ID cannot be empty."
}
```

```json
{
  "job_id": "",
  "message": "Invalid time range for TWAP calculation."
}
```

**Status Code:** `500 Internal Server Error` - Database Error

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "An error occurred: database connection failed"
}
```

#### Processing Flow

1. **Job ID Generation**: A unique job ID is generated based on the request parameters (deterministic in tests, UUID in production)
2. **Database Check**: System checks if a job with the same ID already exists
3. **Status Handling**:
   - If **new**: Job is created and processing begins
   - If **pending**: Returns conflict status
   - If **completed**: Returns success with no reprocessing
   - If **failed**: Job is reset to pending and reprocessed
4. **Asynchronous Processing**: The job is processed in the background
5. **Proving Service**: Request is forwarded to the proving service
6. **On-chain Confirmation**: System waits for `FossilCallbackSuccess` event

---

### Get Job Status

Retrieve the current status of a job.

**Endpoint:** `GET /job_status/{job_id}`

**Authentication:** Not required (Public endpoint)

**Description:** Query the status of a pricing data job. Returns detailed information including on-chain confirmation data when available.

#### Request

**URL Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| job_id | string | Yes | The job ID returned from the pricing data request |

```bash
curl -X GET http://localhost:3000/job_status/550e8400-e29b-41d4-a716-446655440000
```

#### Response

**Status Code:** `200 OK` - Job Found

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending",
  "message": "Job is pending on-chain confirmation via FossilCallbackSuccess event",
  "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
  "expected_timestamp": 1672574400,
  "l1_data": {
    "twap": "0x1234567890abcdef",
    "max_return": "0xabcdef123456",
    "reserve_price": "0xfedcba987654"
  },
  "on_chain_confirmation": {
    "block_number": 123456,
    "transaction_hash": "0xabc123...",
    "event_timestamp": 1672574500
  }
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| job_id | string | Job identifier |
| status | string | Current status: "Pending", "Completed", or "Failed" |
| message | string | Status-specific message |
| vault_address | string | Associated vault address |
| expected_timestamp | integer | Expected completion time (Unix timestamp) |
| l1_data | object | Layer 1 data (when available) |
| l1_data.twap | string | TWAP value as hex string (u256) |
| l1_data.max_return | string | Maximum return as hex string (u128) |
| l1_data.reserve_price | string | Reserve price as hex string (u256) |
| on_chain_confirmation | object | On-chain confirmation details (when available) |
| on_chain_confirmation.block_number | integer | StarkNet block number |
| on_chain_confirmation.transaction_hash | string | Transaction hash |
| on_chain_confirmation.event_timestamp | integer | Event timestamp (Unix) |

#### Status Messages

| Status | Condition | Message |
|--------|-----------|---------|
| Pending | Has vault address | "Job is pending on-chain confirmation via FossilCallbackSuccess event" |
| Pending | No vault address | "Job is pending" |
| Completed | Has L1 data | "Job completed with on-chain confirmation" |
| Completed | No L1 data | "Job completed" |
| Failed | - | "Job failed" |

#### Error Responses

**Status Code:** `404 Not Found` - Job Not Found

```json
{
  "error": "Job not found"
}
```

**Status Code:** `500 Internal Server Error`

```json
{
  "error": "An internal error occurred. Please try again later."
}
```

#### Example - Pending Job

```bash
curl -X GET http://localhost:3000/job_status/550e8400-e29b-41d4-a716-446655440000
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending",
  "message": "Job is pending on-chain confirmation via FossilCallbackSuccess event"
}
```

#### Example - Completed Job

```bash
curl -X GET http://localhost:3000/job_status/550e8400-e29b-41d4-a716-446655440000
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "message": "Job completed with on-chain confirmation",
  "l1_data": {
    "twap": "0x1234567890abcdef",
    "max_return": "0xabcdef123456",
    "reserve_price": "0xfedcba987654"
  },
  "on_chain_confirmation": {
    "block_number": 123456,
    "transaction_hash": "0xabc123...",
    "event_timestamp": 1672574500
  }
}
```

---

### Get Job Result

Retrieve detailed job results with timestamps.

**Endpoint:** `GET /job_result/{job_id}`

**Authentication:** Required

**Description:** Returns comprehensive job information including processing times and detailed results. This endpoint provides more detailed information than the job status endpoint.

#### Request

**Headers:**
```
X-API-Key: your-api-key-here
```

**URL Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| job_id | string | Yes | The job ID to query |

```bash
curl -X GET http://localhost:3000/job_result/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e"
```

#### Response

**Status Code:** `200 OK` - Success

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "result": {
    "twap": 12345.67,
    "max_return": 2345.89,
    "reserve_price": 3456.78
  },
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:32:00Z"
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| job_id | string | Job identifier |
| status | string | Job status enum: "Pending", "Completed", or "Failed" |
| result | object | Calculation results (null if pending or failed) |
| created_at | string | ISO 8601 timestamp when job was created |
| completed_at | string | ISO 8601 timestamp when job completed (null if pending) |

#### Error Responses

**Status Code:** `401 Unauthorized` - Authentication Failed

```json
{
  "error": "Authentication failed: No API key provided in headers"
}
```

**Status Code:** `404 Not Found` - Job Not Found

```json
{
  "error": "Job not found"
}
```

**Status Code:** `500 Internal Server Error`

```json
{
  "error": "An internal error occurred. Please try again later."
}
```

#### Examples

**Completed Job:**
```bash
curl -X GET http://localhost:3000/job_result/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e"
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Completed",
  "result": {
    "twap": 12345.67,
    "max_return": 2345.89,
    "reserve_price": 3456.78
  },
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:32:00Z"
}
```

**Pending Job:**
```bash
curl -X GET http://localhost:3000/job_result/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e"
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Pending",
  "result": null,
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": null
}
```

**Failed Job:**
```bash
curl -X GET http://localhost:3000/job_result/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e"
```

Response:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Failed",
  "result": {
    "error": "Error calling proving service: connection timeout"
  },
  "created_at": "2024-01-15T10:30:00Z",
  "completed_at": "2024-01-15T10:31:00Z"
}
```

---

### Get Batch Job Status

Query the status of multiple jobs in a single request.

**Endpoint:** `POST /batch_job_status`

**Authentication:** Required

**Description:** Efficiently retrieve status information for multiple jobs. Maximum of 100 job IDs per request.

#### Request

**Headers:**
```
Content-Type: application/json
X-API-Key: your-api-key-here
```

**Body:**
```json
{
  "job_ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "550e8400-e29b-41d4-a716-446655440001",
    "550e8400-e29b-41d4-a716-446655440002"
  ]
}
```

**Parameters:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| job_ids | array[string] | Yes | Array of job IDs to query (max 100) |

**Validation Rules:**
- Array must not be empty
- Maximum 100 job IDs per request
- Duplicate IDs are allowed but inefficient

#### Response

**Status Code:** `200 OK` - Success

```json
{
  "jobs": [
    {
      "job_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "Completed",
      "result": {
        "twap": 12345.67,
        "max_return": 2345.89,
        "reserve_price": 3456.78
      },
      "created_at": "2024-01-15T10:30:00Z",
      "completed_at": "2024-01-15T10:32:00Z"
    },
    {
      "job_id": "550e8400-e29b-41d4-a716-446655440001",
      "status": "Pending",
      "result": null,
      "created_at": "2024-01-15T10:35:00Z",
      "completed_at": null
    }
  ],
  "not_found": [
    "550e8400-e29b-41d4-a716-446655440002"
  ]
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| jobs | array | Array of job result objects (same format as `/job_result`) |
| not_found | array[string] | Array of job IDs that were not found |

#### Error Responses

**Status Code:** `400 Bad Request` - Empty Request

```json
{
  "jobs": [],
  "not_found": []
}
```

**Status Code:** `400 Bad Request` - Too Many Jobs

```json
{
  "jobs": [],
  "not_found": ["job1", "job2", ..., "job101"]
}
```

**Status Code:** `401 Unauthorized` - Authentication Failed

```json
{
  "error": "Authentication failed: Invalid API key"
}
```

**Status Code:** `500 Internal Server Error`

```json
{
  "jobs": [],
  "not_found": ["550e8400-e29b-41d4-a716-446655440000", ...]
}
```

#### Examples

**Basic Batch Query:**
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "job_ids": [
      "550e8400-e29b-41d4-a716-446655440000",
      "550e8400-e29b-41d4-a716-446655440001",
      "nonexistent-job-id"
    ]
  }'
```

Response:
```json
{
  "jobs": [
    {
      "job_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "Completed",
      "result": {"twap": 12345.67, "max_return": 2345.89, "reserve_price": 3456.78},
      "created_at": "2024-01-15T10:30:00Z",
      "completed_at": "2024-01-15T10:32:00Z"
    },
    {
      "job_id": "550e8400-e29b-41d4-a716-446655440001",
      "status": "Pending",
      "result": null,
      "created_at": "2024-01-15T10:35:00Z",
      "completed_at": null
    }
  ],
  "not_found": [
    "nonexistent-job-id"
  ]
}
```

**Empty Request (Error):**
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{"job_ids": []}'
```

Response (400 Bad Request):
```json
{
  "jobs": [],
  "not_found": []
}
```

**Too Many Jobs (Error):**
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "job_ids": ["job1", "job2", ..., "job101"]
  }'
```

Response (400 Bad Request):
```json
{
  "jobs": [],
  "not_found": ["job1", "job2", ..., "job101"]
}
```

---

### Webhook Callback

Receive notifications about job processing from external systems.

**Endpoint:** `POST /webhook/{job_id}`

**Authentication:** Not required (Public endpoint)

**Description:** Accepts webhook callbacks from external systems (e.g., PitchLake) to receive notifications about job processing events. Currently logs the callback and returns acknowledgment.

#### Request

**URL Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| job_id | string | Yes | The job ID associated with this callback |

**Headers:**
```
Content-Type: application/json
```

**Body:**
```json
{
  "event": "job_completed",
  "timestamp": "2024-01-15T10:32:00Z",
  "data": {
    "custom_field": "value"
  }
}
```

The body can contain any valid JSON payload. There are no strict requirements on the structure.

```bash
curl -X POST http://localhost:3000/webhook/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{
    "event": "processing_update",
    "status": "in_progress",
    "timestamp": "2024-01-15T10:31:00Z"
  }'
```

#### Response

**Status Code:** `200 OK`

```json
{
  "status": "received",
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| status | string | Always "received" |
| job_id | string | The job ID from the URL |
| timestamp | string | ISO 8601 timestamp of acknowledgment |

#### Example

```bash
curl -X POST http://localhost:3000/webhook/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{
    "event": "job_completed",
    "result": {
      "twap": 12345.67,
      "max_return": 2345.89,
      "reserve_price": 3456.78
    },
    "timestamp": "2024-01-15T10:32:00Z"
  }'
```

Response:
```json
{
  "status": "received",
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

#### Notes

- This endpoint is public and does not require authentication
- The callback payload is logged for debugging
- Future versions may implement callback processing logic
- Consider implementing webhook signature verification for production use

---

## Error Codes

The API uses standard HTTP status codes and returns detailed error messages.

### HTTP Status Codes

| Code | Name | Description |
|------|------|-------------|
| 200 | OK | Request successful |
| 201 | Created | Resource created successfully (new job) |
| 400 | Bad Request | Invalid request parameters |
| 401 | Unauthorized | Authentication failed |
| 404 | Not Found | Resource not found |
| 409 | Conflict | Resource conflict (e.g., job already pending) |
| 500 | Internal Server Error | Server error occurred |

### Error Response Format

All errors return a JSON object with an `error` field:

```json
{
  "error": "Descriptive error message"
}
```

For job-related errors, the response may include additional context:

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Error description",
  "status": "Failed"
}
```

### Common Errors

#### Authentication Errors (401)

| Error Message | Cause | Solution |
|---------------|-------|----------|
| Authentication failed: No API key provided in headers | Missing `X-API-Key` header | Add the header with valid API key |
| Authentication failed: Invalid API key format | Malformed API key | Check API key format |
| Authentication failed: API key not found | Invalid or revoked key | Generate new API key |
| Authentication failed: Database error occurred while validating API key | Database connection issue | Retry request or contact support |

#### Validation Errors (400)

| Error Message | Cause | Solution |
|---------------|-------|----------|
| Program ID cannot be empty | Missing or empty `program_id` | Provide valid program ID |
| Invalid time range for TWAP calculation | start >= end timestamp | Ensure start < end |
| Invalid time range for Max Return calculation | start >= end timestamp | Ensure start < end |
| Invalid time range for Reserve Price calculation | start >= end timestamp | Ensure start < end |

#### Resource Errors (404)

| Error Message | Cause | Solution |
|---------------|-------|----------|
| Job not found | Invalid job ID | Verify job ID is correct |

#### Conflict Errors (409)

| Error Message | Cause | Solution |
|---------------|-------|----------|
| Job is already pending. Use the status endpoint to monitor progress. | Duplicate request for pending job | Wait for job completion or check status |

#### Server Errors (500)

| Error Message | Cause | Solution |
|---------------|-------|----------|
| An error occurred: {details} | Database or internal error | Retry request or contact support |
| An internal error occurred. Please try again later. | Unexpected server error | Retry request or contact support |
| Failed to store API key | Database write error | Retry request or contact support |

### Error Handling Best Practices

1. **Implement Retry Logic**: For 5xx errors, implement exponential backoff
2. **Validate Before Sending**: Validate parameters client-side to avoid 400 errors
3. **Handle Conflicts**: For 409 responses, poll the status endpoint instead of resubmitting
4. **Log API Keys Securely**: Never log API keys in error messages
5. **Monitor Error Rates**: Track error patterns to identify systemic issues

---

## Rate Limiting

**Current Status:** Rate limiting is not currently implemented in the API.

### Future Implementation

When rate limiting is enabled, the following headers will be included in responses:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1672574400
```

### Best Practices

Even without formal rate limits, follow these guidelines:

1. **Batch Requests**: Use `/batch_job_status` for multiple jobs instead of individual requests
2. **Poll Responsibly**: Don't poll status endpoints more than once every 5-10 seconds
3. **Cache Results**: Cache completed job results to avoid redundant queries
4. **Use Webhooks**: When available, use webhooks instead of polling

### Recommendations

- **Normal Usage**: ~10 requests per second per API key
- **Burst Traffic**: Up to 100 requests per second for short periods
- **Batch Queries**: Preferred for 10+ jobs

---

## Common Use Cases

### 1. Submit and Monitor a Single Job

**Step 1: Create API Key**
```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{"name": "my_application"}'
```

**Step 2: Submit Pricing Data Request**
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
```

Save the `job_id` from the response.

**Step 3: Check Status (Public Endpoint)**
```bash
curl -X GET http://localhost:3000/job_status/550e8400-e29b-41d4-a716-446655440000
```

**Step 4: Get Detailed Results (Authenticated)**
```bash
curl -X GET http://localhost:3000/job_result/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e"
```

### 2. Process Multiple Jobs Efficiently

**Submit Multiple Jobs:**
```bash
# Job 1
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'

# Job 2
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672574400, 1672617600],
      "max_return": [1672574400, 1672617600],
      "reserve_price": [1672574400, 1672617600]
    }
  }'
```

**Check All Jobs at Once:**
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "job_ids": [
      "550e8400-e29b-41d4-a716-446655440000",
      "550e8400-e29b-41d4-a716-446655440001"
    ]
  }'
```

### 3. Handle Failed Jobs

When a job fails, the API allows automatic retry:

```bash
# First attempt - job fails
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'

# Check status - shows Failed
curl -X GET http://localhost:3000/job_status/550e8400-e29b-41d4-a716-446655440000

# Resubmit same request - automatically retries failed job
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
# Response: "Previous job request failed. Reprocessing initiated."
```

### 4. Poll for Completion

**Polling Pattern with Exponential Backoff:**

```bash
#!/bin/bash

JOB_ID="550e8400-e29b-41d4-a716-446655440000"
API_KEY="724a0c8d-9fea-4c7c-97a0-1aea894a283e"
DELAY=5

while true; do
  RESPONSE=$(curl -s -X GET http://localhost:3000/job_status/$JOB_ID)
  STATUS=$(echo $RESPONSE | jq -r '.status')

  echo "Current status: $STATUS"

  if [ "$STATUS" = "Completed" ] || [ "$STATUS" = "Failed" ]; then
    # Get detailed results
    curl -X GET http://localhost:3000/job_result/$JOB_ID \
      -H "X-API-Key: $API_KEY"
    break
  fi

  echo "Waiting ${DELAY}s before next check..."
  sleep $DELAY

  # Exponential backoff (max 60s)
  DELAY=$((DELAY * 2))
  if [ $DELAY -gt 60 ]; then
    DELAY=60
  fi
done
```

### 5. Idempotent Request Pattern

The API is idempotent - submitting the same request multiple times is safe:

```bash
# First request - creates job
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
# Response: 201 Created - "New job request registered and processing initiated."

# Second request (same parameters) - returns existing job
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -H "X-API-Key: 724a0c8d-9fea-4c7c-97a0-1aea894a283e" \
  -d '{
    "program_id": "PITCHLAKE_V1",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "params": {
      "twap": [1672531200, 1672574400],
      "max_return": [1672531200, 1672574400],
      "reserve_price": [1672531200, 1672574400]
    }
  }'
# Response: 409 Conflict - "Job is already pending. Use the status endpoint to monitor progress."
```

### 6. Integration with External Systems

**Using Webhooks for Notifications:**

Configure your external system to call the webhook endpoint when job events occur:

```bash
# External system sends webhook notification
curl -X POST http://localhost:3000/webhook/550e8400-e29b-41d4-a716-446655440000 \
  -H "Content-Type: application/json" \
  -d '{
    "event": "job_completed",
    "source": "pitchlake",
    "timestamp": "2024-01-15T10:32:00Z",
    "result": {
      "twap": 12345.67,
      "max_return": 2345.89,
      "reserve_price": 3456.78
    }
  }'
```

---

## Next Steps

### Getting Started

1. **Local Development**: Follow the [Getting Started Guide](/docs/getting-started/quickstart.md)
2. **Generate API Key**: Use the `/api_key` endpoint or CLI tool
3. **Submit Test Request**: Try the `/pricing_data` endpoint with sample data
4. **Monitor Jobs**: Use `/job_status` or `/batch_job_status` to track progress

### Additional Resources

- [Architecture Overview](/docs/architecture/system-overview.md) - Understand the system design
- [Development Guide](/docs/contributing/development-guide.md) - Set up local development
- [Database Schema](/docs/crates/fossil-api/db-access.md) - Database structure and migrations
- [Event Monitoring](/docs/architecture/event-monitoring.md) - On-chain event tracking

### Support

- **GitHub Issues**: [fossil-monorepo/issues](https://github.com/your-org/fossil-monorepo/issues)
- **Documentation**: [/docs](/docs)
- **API Status**: Check `/health` endpoint

### Production Considerations

When deploying to production:

1. **Secure API Keys**: Store keys in secure vault (AWS Secrets Manager, HashiCorp Vault, etc.)
2. **Enable HTTPS**: Use TLS/SSL certificates for all API communication
3. **Configure CORS**: Set appropriate `ALLOWED_ORIGINS` environment variable
4. **Monitor Logs**: Set up structured logging and monitoring
5. **Database Backups**: Configure automated PostgreSQL backups
6. **Error Tracking**: Integrate error tracking (Sentry, Rollbar, etc.)
7. **Rate Limiting**: Implement rate limiting as needed
8. **API Versioning**: Plan for API version management

### Environment Variables

Key environment variables for API configuration:

```bash
# Database
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:pass@host:5432/db
INDEXER_DATABASE_URL=postgresql://user:pass@host:5432/indexer_db

# StarkNet
STARKNET_RPC_URL=https://starknet-mainnet.infura.io/v3/YOUR-PROJECT-ID

# Services
PROVING_SERVICE_URL=https://proving-service.your-domain.com

# CORS
ALLOWED_ORIGINS=https://app.your-domain.com,https://www.your-domain.com

# Event Monitor
EVENT_MONITOR_POLLING_INTERVAL=30
EVENT_MONITOR_BLOCKS_PER_SCAN=1000
```

See [.env.example](/home/ametel/source/fossil-monorepo/.env.example) for complete configuration options.
