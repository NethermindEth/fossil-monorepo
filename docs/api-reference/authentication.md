# Authentication Reference

## Overview

The Fossil API uses API key-based authentication to secure access to protected endpoints. API keys are UUID v4 tokens that are validated against the database on each request. Authentication is implemented using a middleware layer that intercepts requests to protected endpoints and validates the `X-API-Key` header.

### Key Features

- **Simple API Key Authentication**: Bearer-token style authentication using unique UUID v4 keys
- **Header-based**: Keys are passed via the `X-API-Key` HTTP header
- **Database-backed**: Keys are stored and validated against PostgreSQL
- **Middleware Protection**: Automatic authentication for designated endpoints
- **Public & Protected Routes**: Separation between public and authenticated endpoints

### Authentication Architecture

```
Client Request
    |
    v
[X-API-Key Header]
    |
    v
[Auth Middleware]
    |
    +---> Extract API Key from headers
    |
    +---> Validate against database
    |
    +---> [Valid] --> Allow request to proceed
    |
    +---> [Invalid] --> Return 401 Unauthorized
```

## Getting Started

### Quick Start

1. **Create an API Key**:
   ```bash
   curl -X POST http://localhost:3000/api_key \
     -H "Content-Type: application/json" \
     -d '{"name": "my-application"}'
   ```

   Response:
   ```json
   {
     "api_key": "a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7"
   }
   ```

2. **Use the API Key**:
   ```bash
   curl -X POST http://localhost:3000/pricing_data \
     -H "X-API-Key: a1b2c3d4-e5f6-4789-a0b1-c2d3e4f5g6h7" \
     -H "Content-Type: application/json" \
     -d '{"vault_address": "0x123...", "expected_timestamp": 1234567890}'
   ```

## API Key Management

### Creating API Keys

API keys can be created in two ways:

#### 1. Via REST API (Recommended)

**Endpoint**: `POST /api_key`

**Request**:
```bash
curl -X POST http://localhost:3000/api_key \
  -H "Content-Type: application/json" \
  -d '{
    "name": "production-app"
  }'
```

**Response**:
```json
{
  "api_key": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Request Schema**:
```json
{
  "name": "string (required)"
}
```

**Response Schema**:
```json
{
  "api_key": "string (UUID v4 format)"
}
```

**Status Codes**:
- `200 OK`: API key created successfully
- `500 Internal Server Error`: Database error occurred

#### 2. Via Command Line Script

For administrative purposes, you can create API keys directly via a CLI script:

```bash
cd fossil-api/crates/server
cargo run --bin create_api_key -- "my-application-name"
```

**Output**:
```
API Key Created: 550e8400-e29b-41d4-a716-446655440000
```

**Important**: Save the API key immediately. It cannot be retrieved later.

### Key Format and Structure

- **Format**: UUID v4 (RFC 4122)
- **Length**: 36 characters (32 hex digits + 4 hyphens)
- **Example**: `550e8400-e29b-41d4-a716-446655440000`
- **Character Set**: `0-9`, `a-f`, `-`

**Valid API Key Pattern**:
```regex
^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$
```

### Key Storage

API keys are stored in the PostgreSQL database with the following schema:

```sql
CREATE TABLE IF NOT EXISTS public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);
```

**Fields**:
- `id`: Auto-incrementing primary key
- `key`: The API key (unique constraint enforced)
- `name`: Human-readable identifier for the key
- `created_at`: Timestamp when the key was created

**Important Security Notes**:

1. **No Hashing**: Currently, API keys are stored in plaintext in the database. This is acceptable for internal use but should be enhanced with hashing for production environments.

2. **Database Access**: Ensure your database has proper access controls and is not publicly accessible.

3. **Unique Constraint**: The database enforces uniqueness on the `key` field, preventing duplicate keys.

### Key Rotation

To rotate an API key:

1. **Create a new API key**:
   ```bash
   curl -X POST http://localhost:3000/api_key \
     -H "Content-Type: application/json" \
     -d '{"name": "production-app-v2"}'
   ```

2. **Update your application** to use the new key

3. **Test** that the new key works correctly

4. **Deprecate the old key** (manual database deletion currently required):
   ```sql
   DELETE FROM api_keys WHERE key = 'old-key-value';
   ```

**Best Practice**: Implement a grace period where both old and new keys are valid to ensure zero-downtime rotation.

### Revoking Keys

Currently, key revocation requires direct database access:

```sql
-- Delete a specific API key
DELETE FROM api_keys WHERE key = '550e8400-e29b-41d4-a716-446655440000';

-- Delete by name
DELETE FROM api_keys WHERE name = 'compromised-app';

-- List all API keys (for auditing)
SELECT key, name, created_at FROM api_keys ORDER BY created_at DESC;
```

**Security Warning**: If a key is compromised, revoke it immediately.

## Using API Keys

### Header Format

All authenticated requests must include the `X-API-Key` header:

```
X-API-Key: your-api-key-here
```

**Important**:
- Header name is case-insensitive (`x-api-key`, `X-Api-Key`, `X-API-KEY` all work)
- No prefix required (no "Bearer" or other scheme)
- The key must be the complete UUID v4 value

### Example Requests

#### cURL

**Basic Request**:
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: 550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "vault_address": "0x1234567890abcdef",
    "expected_timestamp": 1699564800
  }'
```

**Batch Status Request**:
```bash
curl -X POST http://localhost:3000/batch_job_status \
  -H "X-API-Key: 550e8400-e29b-41d4-a716-446655440000" \
  -H "Content-Type: application/json" \
  -d '{
    "job_ids": ["job1", "job2", "job3"]
  }'
```

#### Python (requests)

```python
import requests
import os

# Store API key in environment variable
API_KEY = os.getenv('FOSSIL_API_KEY')
BASE_URL = 'http://localhost:3000'

headers = {
    'X-API-Key': API_KEY,
    'Content-Type': 'application/json'
}

# Get pricing data
response = requests.post(
    f'{BASE_URL}/pricing_data',
    headers=headers,
    json={
        'vault_address': '0x1234567890abcdef',
        'expected_timestamp': 1699564800
    }
)

if response.status_code == 200:
    data = response.json()
    print(f"Job ID: {data['job_id']}")
else:
    print(f"Error: {response.status_code} - {response.text}")
```

#### JavaScript (fetch)

```javascript
const FOSSIL_API_KEY = process.env.FOSSIL_API_KEY;
const BASE_URL = 'http://localhost:3000';

async function getPricingData(vaultAddress, timestamp) {
  const response = await fetch(`${BASE_URL}/pricing_data`, {
    method: 'POST',
    headers: {
      'X-API-Key': FOSSIL_API_KEY,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      vault_address: vaultAddress,
      expected_timestamp: timestamp
    })
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(`API Error: ${error.error}`);
  }

  return await response.json();
}

// Usage
getPricingData('0x1234567890abcdef', 1699564800)
  .then(data => console.log('Job ID:', data.job_id))
  .catch(err => console.error('Error:', err));
```

#### JavaScript (axios)

```javascript
const axios = require('axios');

const client = axios.create({
  baseURL: 'http://localhost:3000',
  headers: {
    'X-API-Key': process.env.FOSSIL_API_KEY,
    'Content-Type': 'application/json'
  }
});

// Get pricing data
client.post('/pricing_data', {
  vault_address: '0x1234567890abcdef',
  expected_timestamp: 1699564800
})
  .then(response => {
    console.log('Job ID:', response.data.job_id);
  })
  .catch(error => {
    if (error.response) {
      console.error('Error:', error.response.status, error.response.data);
    } else {
      console.error('Error:', error.message);
    }
  });
```

#### Rust (reqwest)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize)]
struct PricingRequest {
    vault_address: String,
    expected_timestamp: i64,
}

#[derive(Deserialize)]
struct PricingResponse {
    job_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("FOSSIL_API_KEY")?;
    let base_url = "http://localhost:3000";

    let client = Client::new();

    let response = client
        .post(&format!("{}/pricing_data", base_url))
        .header("X-API-Key", api_key)
        .json(&PricingRequest {
            vault_address: "0x1234567890abcdef".to_string(),
            expected_timestamp: 1699564800,
        })
        .send()
        .await?;

    if response.status().is_success() {
        let data: PricingResponse = response.json().await?;
        println!("Job ID: {}", data.job_id);
    } else {
        let error_text = response.text().await?;
        eprintln!("Error: {}", error_text);
    }

    Ok(())
}
```

### Public vs Protected Endpoints

The API has two types of endpoints:

#### Public Endpoints (No Authentication Required)

These endpoints do not require an API key:

- `GET /health` - Health check endpoint
- `POST /api_key` - Create new API key
- `GET /job_status/{job_id}` - Get job status by ID
- `POST /webhook/{job_id}` - Job callback webhook

**Example**:
```bash
# No X-API-Key header needed
curl http://localhost:3000/health
```

#### Protected Endpoints (Authentication Required)

These endpoints require a valid API key in the `X-API-Key` header:

- `POST /pricing_data` - Submit pricing data request
- `GET /job_result/{job_id}` - Get job result
- `POST /batch_job_status` - Get status for multiple jobs

**Example**:
```bash
# X-API-Key header required
curl -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: your-api-key"
```

## Security Best Practices

### 1. Key Storage

**DO**:
- Store API keys in environment variables
- Use a secrets management system (AWS Secrets Manager, HashiCorp Vault, etc.)
- Keep keys in `.env.local` files (never committed to git)
- Use different keys for each environment (dev, staging, production)

**DON'T**:
- Hard-code API keys in source code
- Commit API keys to version control
- Share keys via email or chat
- Use the same key across multiple environments

**Example `.env.local`**:
```bash
# Fossil API Configuration
FOSSIL_API_KEY=550e8400-e29b-41d4-a716-446655440000
FOSSIL_API_URL=http://localhost:3000
```

**Example `.gitignore`**:
```
# Environment files
.env.local
.env.*.local
.env.production

# Never commit these
**/api_keys.txt
**/secrets/
```

### 2. Key Rotation Schedule

Implement regular key rotation to minimize the impact of potential compromises:

- **Development**: Rotate monthly or as needed
- **Staging**: Rotate quarterly
- **Production**: Rotate every 90 days minimum

**Rotation Checklist**:
1. Generate new API key
2. Update secrets management system
3. Deploy updated configuration to all services
4. Verify new key works correctly
5. Monitor for errors
6. Wait 24-48 hours (grace period)
7. Revoke old key
8. Confirm old key is no longer in use

### 3. Environment-Specific Keys

Use different API keys for each environment:

```bash
# Development
FOSSIL_API_KEY=dev-550e8400-e29b-41d4-a716-446655440000

# Staging
FOSSIL_API_KEY=staging-7c9e6679-7425-40de-944b-e07fc1f90ae7

# Production
FOSSIL_API_KEY=prod-9b2c7e25-7425-40de-944b-e07fc1f90ae7
```

**Benefits**:
- Easier to track usage per environment
- Limit blast radius if a key is compromised
- Enable per-environment rate limiting
- Simplify debugging and monitoring

### 4. Don't Commit Keys to Git

**Prevention Techniques**:

1. **Use `.gitignore`**:
   ```
   .env
   .env.local
   .env.*.local
   ```

2. **Use environment variables**:
   ```bash
   export FOSSIL_API_KEY="your-key-here"
   ```

3. **Use git-secrets** (pre-commit hook):
   ```bash
   brew install git-secrets
   git secrets --install
   git secrets --register-aws
   ```

4. **Scan for secrets** with tools like:
   - [gitleaks](https://github.com/gitleaks/gitleaks)
   - [truffleHog](https://github.com/trufflesecurity/truffleHog)
   - [detect-secrets](https://github.com/Yelp/detect-secrets)

**If you accidentally commit a key**:
1. Revoke it immediately
2. Generate a new key
3. Update all services
4. Consider using `git-filter-repo` to remove it from history
5. Audit access logs for unauthorized usage

### 5. Rate Limiting Per Key

While not currently implemented, you should track usage per API key for:

- **Abuse Detection**: Identify unusual usage patterns
- **Cost Allocation**: Track usage per application/customer
- **Capacity Planning**: Understand peak usage times
- **Security Monitoring**: Detect potential attacks

**Recommended Implementation**:
```sql
-- Add rate limiting fields to api_keys table
ALTER TABLE api_keys ADD COLUMN rate_limit_per_minute INTEGER DEFAULT 60;
ALTER TABLE api_keys ADD COLUMN rate_limit_per_day INTEGER DEFAULT 10000;

-- Create usage tracking table
CREATE TABLE api_key_usage (
    id SERIAL PRIMARY KEY,
    api_key_id INTEGER REFERENCES api_keys(id),
    endpoint TEXT NOT NULL,
    request_timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    response_status INTEGER,
    INDEX idx_key_timestamp (api_key_id, request_timestamp)
);
```

### 6. HTTPS in Production

**CRITICAL**: Always use HTTPS in production environments.

```bash
# Development (HTTP OK)
FOSSIL_API_URL=http://localhost:3000

# Production (HTTPS REQUIRED)
FOSSIL_API_URL=https://api.fossil.example.com
```

**Why**:
- API keys in headers are transmitted in plaintext over HTTP
- Man-in-the-middle attacks can intercept keys
- HTTPS encrypts all traffic including headers

### 7. Monitoring and Auditing

Track API key usage for security and compliance:

```sql
-- Audit query: Recent API key usage
SELECT
    ak.name,
    ak.key,
    COUNT(*) as request_count,
    MAX(created_at) as last_used
FROM api_keys ak
GROUP BY ak.name, ak.key
ORDER BY last_used DESC;

-- Detect old/unused keys
SELECT
    name,
    created_at,
    EXTRACT(DAYS FROM NOW() - created_at) as days_old
FROM api_keys
WHERE created_at < NOW() - INTERVAL '90 days'
ORDER BY created_at ASC;
```

## Authentication Flow

### Request Flow Diagram

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       │ HTTP Request
       │ Header: X-API-Key: xxx-xxx-xxx
       │
       v
┌─────────────────────────────────────┐
│         Axum HTTP Server            │
└──────┬──────────────────────────────┘
       │
       │ Route to endpoint
       │
       v
┌─────────────────────────────────────┐
│    Auth Middleware Layer            │
│  (simple_apikey_auth)               │
└──────┬──────────────────────────────┘
       │
       ├─► Extract X-API-Key header
       │   ├─ Missing? → 401 Unauthorized
       │   └─ Invalid format? → 401 Unauthorized
       │
       ├─► Validate against database
       │   ├─ Query: SELECT * FROM api_keys WHERE key = ?
       │   ├─ Not found? → 401 Unauthorized
       │   └─ Database error? → 401 Unauthorized
       │
       │ Valid API key ✓
       │
       v
┌─────────────────────────────────────┐
│      Protected Handler              │
│  (pricing_data, job_result, etc.)   │
└──────┬──────────────────────────────┘
       │
       │ Process request
       │
       v
┌─────────────────────────────────────┐
│         Response to Client          │
└─────────────────────────────────────┘
```

### Middleware Implementation Overview

The authentication is implemented in the `simple_apikey_auth` middleware:

**File**: `fossil-api/crates/server/src/middlewares/auth.rs`

**Key Components**:

1. **Extract API Key** (`extract_api_key`):
   - Reads the `x-api-key` header from the request
   - Validates header exists and is valid UTF-8
   - Returns 401 if missing or malformed

2. **Validate API Key** (`validate_api_key`):
   - Queries the database for the API key
   - Uses `find_api_key` from `db_access::auth`
   - Returns 401 if key not found or database error

3. **Create Error Response** (`create_auth_error`):
   - Generates standardized 401 Unauthorized responses
   - Includes error details in JSON format
   - Logs authentication failures

**Code Flow**:
```rust
pub async fn simple_apikey_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract API key from headers
    let api_key_str = match extract_api_key(&headers) {
        Ok(key) => key,
        Err(response) => return Ok(*response),
    };

    // 2. Validate against database
    match validate_api_key(&state, api_key_str).await {
        Ok(()) => {
            // 3. Allow request to proceed
            Ok(next.run(request).await)
        }
        Err(response) => Ok(*response),
    }
}
```

### Error Responses

#### 401 Unauthorized - Missing API Key

**Request**:
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "Content-Type: application/json" \
  -d '{"vault_address": "0x123"}'
```

**Response**:
```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json

{
  "error": "Authentication failed: No API key provided in headers"
}
```

#### 401 Unauthorized - Invalid API Key Format

**Request**:
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: invalid-key-with-unicode-\xff\xfe" \
  -H "Content-Type: application/json" \
  -d '{"vault_address": "0x123"}'
```

**Response**:
```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json

{
  "error": "Authentication failed: Invalid API key format"
}
```

#### 401 Unauthorized - API Key Not Found

**Request**:
```bash
curl -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: 00000000-0000-0000-0000-000000000000" \
  -H "Content-Type: application/json" \
  -d '{"vault_address": "0x123"}'
```

**Response**:
```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json

{
  "error": "Authentication failed: API key not found"
}
```

#### 401 Unauthorized - Database Error

**Response**:
```http
HTTP/1.1 401 Unauthorized
Content-Type: application/json

{
  "error": "Authentication failed: Database error occurred while validating API key"
}
```

**Note**: 403 Forbidden is not currently used. All authentication failures return 401 Unauthorized.

## Troubleshooting

### Common Authentication Errors

#### Problem: "No API key provided in headers"

**Cause**: The `X-API-Key` header is missing from the request.

**Solution**:
```bash
# Incorrect (missing header)
curl -X POST http://localhost:3000/pricing_data

# Correct
curl -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: your-api-key-here"
```

**Check**:
```bash
# Verify headers are being sent
curl -v -X POST http://localhost:3000/pricing_data \
  -H "X-API-Key: your-api-key-here" 2>&1 | grep "X-API-Key"
```

#### Problem: "Invalid API key format"

**Cause**: The API key contains invalid characters or encoding.

**Common Issues**:
- Key copied with extra whitespace
- Non-UTF8 characters in the header
- URL encoding issues

**Solution**:
```bash
# Incorrect (extra spaces)
curl -H "X-API-Key:  550e8400-e29b-41d4-a716-446655440000  "

# Correct (no extra spaces)
curl -H "X-API-Key: 550e8400-e29b-41d4-a716-446655440000"
```

**Check**:
```bash
# Verify the key format
echo "550e8400-e29b-41d4-a716-446655440000" | \
  grep -E '^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'
```

#### Problem: "API key not found"

**Cause**: The API key does not exist in the database.

**Debug Steps**:

1. **Verify the key exists**:
   ```sql
   SELECT key, name, created_at
   FROM api_keys
   WHERE key = '550e8400-e29b-41d4-a716-446655440000';
   ```

2. **Check for typos**:
   ```bash
   # List all keys
   psql -d fossil_db -c "SELECT key, name FROM api_keys;"
   ```

3. **Create a new key if needed**:
   ```bash
   curl -X POST http://localhost:3000/api_key \
     -H "Content-Type: application/json" \
     -d '{"name": "test-key"}'
   ```

#### Problem: "Database error occurred"

**Cause**: Cannot connect to the database or query failed.

**Debug Steps**:

1. **Check database connection**:
   ```bash
   psql $OFFCHAIN_PROCESSOR_DATABASE_URL -c "SELECT 1;"
   ```

2. **Verify api_keys table exists**:
   ```sql
   \dt api_keys
   SELECT COUNT(*) FROM api_keys;
   ```

3. **Check database logs**:
   ```bash
   # Check PostgreSQL logs
   tail -f /var/log/postgresql/postgresql-*.log
   ```

4. **Verify environment variables**:
   ```bash
   echo $OFFCHAIN_PROCESSOR_DATABASE_URL
   ```

#### Problem: Headers not being sent (CORS issue)

**Cause**: Browser CORS policies blocking custom headers.

**Solution**: Ensure CORS is configured to allow the `X-API-Key` header:

```rust
// In fossil-api/crates/server/src/lib.rs
let cors_layer = CorsLayer::new()
    .allow_origin(AllowOrigin::list(allowed_origins))
    .allow_methods(AllowMethods::any())
    .allow_headers(AllowHeaders::any())  // ← Allows X-API-Key
    .max_age(Duration::from_secs(3600));
```

**Check**:
```bash
# Verify CORS headers in response
curl -v -X OPTIONS http://localhost:3000/pricing_data \
  -H "Origin: http://localhost:3001" \
  -H "Access-Control-Request-Headers: x-api-key" 2>&1 | \
  grep -i "access-control"
```

#### Problem: Using the wrong endpoint

**Cause**: Trying to use authentication on a public endpoint, or vice versa.

**Check the endpoint type**:

**Public Endpoints** (no auth):
- `/health`
- `/api_key`
- `/job_status/{job_id}`
- `/webhook/{job_id}`

**Protected Endpoints** (require auth):
- `/pricing_data`
- `/job_result/{job_id}`
- `/batch_job_status`

**Solution**:
```bash
# Public endpoint - no auth needed
curl http://localhost:3000/health

# Protected endpoint - auth required
curl -H "X-API-Key: your-key" http://localhost:3000/pricing_data
```

### Debugging Tips

#### Enable Debug Logging

Set the `RUST_LOG` environment variable to see detailed authentication logs:

```bash
RUST_LOG=debug cargo run

# Or for specific modules
RUST_LOG=server::middlewares::auth=debug,db_access::auth=debug cargo run
```

**Log Output**:
```
DEBUG server::middlewares::auth: Attempting authentication with API key
DEBUG server::middlewares::auth: Received API key: 550e8400-e29b-41d4-a716-446655440000
DEBUG db_access::auth: Searching for API key: 550e8400-e29b-41d4-a716-446655440000
DEBUG db_access::auth: Found API key: ApiKey { key: "...", name: Some("test") }
INFO  server::middlewares::auth: Authentication successful
```

#### Test Authentication in Isolation

Create a simple test script:

```bash
#!/bin/bash
set -e

API_KEY="$1"
BASE_URL="${2:-http://localhost:3000}"

echo "Testing authentication with key: ${API_KEY:0:8}..."

# Test 1: Health check (no auth needed)
echo -n "Test 1 - Public endpoint: "
curl -s -w "\n%{http_code}\n" "$BASE_URL/health" | tail -1

# Test 2: Protected endpoint with auth
echo -n "Test 2 - Protected endpoint with auth: "
curl -s -w "\n%{http_code}\n" \
  -H "X-API-Key: $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"vault_address":"0x123","expected_timestamp":1}' \
  "$BASE_URL/pricing_data" | tail -1

# Test 3: Protected endpoint without auth
echo -n "Test 3 - Protected endpoint without auth: "
curl -s -w "\n%{http_code}\n" \
  -H "Content-Type: application/json" \
  -d '{"vault_address":"0x123","expected_timestamp":1}' \
  "$BASE_URL/pricing_data" | tail -1

echo "Done!"
```

**Usage**:
```bash
chmod +x test_auth.sh
./test_auth.sh "your-api-key-here"
```

#### Verify Database State

```sql
-- Check all API keys
SELECT
    id,
    key,
    name,
    created_at,
    AGE(NOW(), created_at) as age
FROM api_keys
ORDER BY created_at DESC;

-- Check for duplicate keys (should be none)
SELECT key, COUNT(*) as count
FROM api_keys
GROUP BY key
HAVING COUNT(*) > 1;

-- Check for keys with invalid format
SELECT key, name
FROM api_keys
WHERE key !~ '^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$';
```

## Code Examples

### Complete Integration Example

Here's a complete example of a Python client using the Fossil API with authentication:

```python
#!/usr/bin/env python3
"""
Fossil API Client Example
Demonstrates API key authentication and request handling.
"""

import os
import sys
import time
import requests
from typing import Optional, Dict, Any


class FossilAPIClient:
    """Client for interacting with the Fossil API."""

    def __init__(self, api_key: str, base_url: str = "http://localhost:3000"):
        """
        Initialize the Fossil API client.

        Args:
            api_key: Your Fossil API key
            base_url: Base URL of the Fossil API (default: http://localhost:3000)
        """
        self.api_key = api_key
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            'X-API-Key': self.api_key,
            'Content-Type': 'application/json'
        })

    def health_check(self) -> Dict[str, Any]:
        """
        Check the health of the API.

        Returns:
            Health status response
        """
        response = self.session.get(f'{self.base_url}/health')
        response.raise_for_status()
        return response.json()

    def create_pricing_request(
        self,
        vault_address: str,
        expected_timestamp: int
    ) -> Dict[str, Any]:
        """
        Create a new pricing data request.

        Args:
            vault_address: The vault address
            expected_timestamp: Expected timestamp for the request

        Returns:
            Response containing job_id
        """
        payload = {
            'vault_address': vault_address,
            'expected_timestamp': expected_timestamp
        }

        response = self.session.post(
            f'{self.base_url}/pricing_data',
            json=payload
        )
        response.raise_for_status()
        return response.json()

    def get_job_status(self, job_id: str) -> Dict[str, Any]:
        """
        Get the status of a job.

        Args:
            job_id: The job ID to check

        Returns:
            Job status information
        """
        response = self.session.get(f'{self.base_url}/job_status/{job_id}')
        response.raise_for_status()
        return response.json()

    def get_job_result(self, job_id: str) -> Dict[str, Any]:
        """
        Get the result of a completed job.

        Args:
            job_id: The job ID to retrieve

        Returns:
            Job result data
        """
        response = self.session.get(f'{self.base_url}/job_result/{job_id}')
        response.raise_for_status()
        return response.json()

    def get_batch_job_status(self, job_ids: list[str]) -> Dict[str, Any]:
        """
        Get the status of multiple jobs.

        Args:
            job_ids: List of job IDs to check

        Returns:
            Batch status information
        """
        response = self.session.post(
            f'{self.base_url}/batch_job_status',
            json={'job_ids': job_ids}
        )
        response.raise_for_status()
        return response.json()

    def wait_for_job(
        self,
        job_id: str,
        timeout: int = 300,
        poll_interval: int = 5
    ) -> Dict[str, Any]:
        """
        Wait for a job to complete.

        Args:
            job_id: The job ID to wait for
            timeout: Maximum time to wait in seconds (default: 300)
            poll_interval: Time between status checks in seconds (default: 5)

        Returns:
            Final job status

        Raises:
            TimeoutError: If job doesn't complete within timeout
        """
        start_time = time.time()

        while True:
            status = self.get_job_status(job_id)

            if status['status'] in ['Completed', 'Failed']:
                return status

            elapsed = time.time() - start_time
            if elapsed > timeout:
                raise TimeoutError(
                    f"Job {job_id} did not complete within {timeout} seconds"
                )

            time.sleep(poll_interval)


def main():
    """Example usage of the Fossil API client."""

    # Get API key from environment
    api_key = os.getenv('FOSSIL_API_KEY')
    if not api_key:
        print("Error: FOSSIL_API_KEY environment variable not set")
        sys.exit(1)

    # Initialize client
    client = FossilAPIClient(api_key)

    try:
        # 1. Health check
        print("Checking API health...")
        health = client.health_check()
        print(f"API Status: {health}")

        # 2. Create pricing request
        print("\nCreating pricing request...")
        result = client.create_pricing_request(
            vault_address="0x1234567890abcdef",
            expected_timestamp=int(time.time())
        )
        job_id = result['job_id']
        print(f"Job created: {job_id}")

        # 3. Wait for job completion
        print("\nWaiting for job to complete...")
        final_status = client.wait_for_job(job_id, timeout=300)
        print(f"Job completed with status: {final_status['status']}")

        # 4. Get job result
        if final_status['status'] == 'Completed':
            print("\nRetrieving job result...")
            result = client.get_job_result(job_id)
            print(f"Result: {result}")

    except requests.exceptions.HTTPError as e:
        print(f"HTTP Error: {e}")
        if e.response is not None:
            print(f"Response: {e.response.text}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()
```

**Usage**:
```bash
# Set your API key
export FOSSIL_API_KEY="550e8400-e29b-41d4-a716-446655440000"

# Run the example
python fossil_client.py
```

### Key Rotation Workflow Example

```bash
#!/bin/bash
# rotate_api_key.sh - Automated API key rotation script

set -e

API_URL="${API_URL:-http://localhost:3000}"
OLD_KEY="${FOSSIL_API_KEY}"
KEY_NAME="${1:-production-app}"

echo "Starting API key rotation for: $KEY_NAME"

# Step 1: Create new API key
echo "Creating new API key..."
NEW_KEY_RESPONSE=$(curl -s -X POST "$API_URL/api_key" \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"$KEY_NAME-rotated-$(date +%Y%m%d)\"}")

NEW_KEY=$(echo "$NEW_KEY_RESPONSE" | jq -r '.api_key')

if [ -z "$NEW_KEY" ] || [ "$NEW_KEY" == "null" ]; then
  echo "Failed to create new API key"
  exit 1
fi

echo "New API key created: ${NEW_KEY:0:8}..."

# Step 2: Test new API key
echo "Testing new API key..."
TEST_RESPONSE=$(curl -s -w "%{http_code}" -o /dev/null \
  -H "X-API-Key: $NEW_KEY" \
  -H "Content-Type: application/json" \
  -d '{"vault_address":"0x123","expected_timestamp":1}' \
  "$API_URL/pricing_data")

if [ "$TEST_RESPONSE" != "200" ]; then
  echo "New API key test failed with status: $TEST_RESPONSE"
  exit 1
fi

echo "New API key validated successfully"

# Step 3: Update secrets (example with AWS Secrets Manager)
if command -v aws &> /dev/null; then
  echo "Updating AWS Secrets Manager..."
  aws secretsmanager update-secret \
    --secret-id "fossil-api/key" \
    --secret-string "$NEW_KEY"
  echo "AWS Secrets Manager updated"
fi

# Step 4: Output new key for manual update
echo ""
echo "==================================="
echo "API KEY ROTATION SUCCESSFUL"
echo "==================================="
echo "New API Key: $NEW_KEY"
echo ""
echo "Next steps:"
echo "1. Update your application configuration"
echo "2. Deploy updated configuration"
echo "3. Monitor for errors (24-48 hours)"
echo "4. Revoke old key: DELETE FROM api_keys WHERE key='$OLD_KEY';"
echo "==================================="

# Optional: Save to .env.local
if [ -w .env.local ]; then
  sed -i.bak "s/FOSSIL_API_KEY=.*/FOSSIL_API_KEY=$NEW_KEY/" .env.local
  echo "Updated .env.local (backup saved as .env.local.bak)"
fi
```

**Usage**:
```bash
chmod +x rotate_api_key.sh
./rotate_api_key.sh "production-app"
```

---

## Additional Resources

- [Fossil API Architecture](../architecture/fossil-api.md)
- [Getting Started Guide](../getting-started/installation.md)
- [Database Management](../guides/database-management.md)
- [Running Services](../guides/running-services.md)

## Support

For authentication issues or questions:

1. Check the [Troubleshooting](#troubleshooting) section above
2. Enable debug logging to see detailed authentication flow
3. Verify your database connection and API key existence
4. Review the [middleware implementation](../../fossil-api/crates/server/src/middlewares/auth.rs) for technical details

---

**Last Updated**: 2025-10-06
