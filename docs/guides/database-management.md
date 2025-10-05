# Database Management Guide

This guide covers database setup, migrations, backups, and troubleshooting for the Fossil monorepo's PostgreSQL databases.

## Table of Contents
- [Database Overview](#database-overview)
- [Database Schema](#database-schema)
- [Running Migrations](#running-migrations)
- [Creating New Migrations](#creating-new-migrations)
- [Database Backups](#database-backups)
- [Connection Management](#connection-management)
- [Common Operations](#common-operations)
- [Troubleshooting](#troubleshooting)

## Database Overview

The Fossil monorepo uses three separate PostgreSQL databases, each serving a distinct purpose:

### 1. Fossil API Database

**Environment Variable:** `OFFCHAIN_PROCESSOR_DATABASE_URL`

**Purpose:** Main application database (read/write) for the Fossil API service

**Schema Contents:**
- API key management and authentication
- Job request tracking and status
- Pricing data computation results
- Vault event tracking and L1 data

**Default Ports:**
- Local development: `localhost:5434`
- Docker: `fossil_api_db:5432`

**Connection String Format:**
```bash
# Local development
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5434/postgres

# Docker development
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@fossil_api_db:5432/postgres

# Production (managed database)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:password@db.example.com:5432/fossil_api
```

### 2. Indexer Database

**Environment Variable:** `INDEXER_DATABASE_URL`

**Purpose:** Read-only database populated by the Fossil indexer service

**Schema Contents:**
- Historical blockchain data (Ethereum L1)
- Block headers with gas pricing data
- Transaction and event data

**Default Ports:**
- Local development: `localhost:5433`
- Docker: `indexer_db:5432`

**Connection String Format:**
```bash
# Local development
INDEXER_DATABASE_URL=postgresql://postgres:postgres@localhost:5433/postgres

# Docker development
INDEXER_DATABASE_URL=postgresql://postgres:postgres@indexer_db:5432/postgres
```

**Note:** This database is managed by the fossil-indexer service and should be treated as read-only by other services.

### 3. Proving Service Database

**Environment Variable:** `PROVING_SERVICE_DATABASE_URL`

**Purpose:** Job tracking and proof storage for the proving service

**Schema Contents:**
- Proof generation job queue
- Proof metadata and verification results
- Block header data for proof generation

**Default Ports:**
- Local development: `localhost:5435`
- Docker: `proving_service_db:5432`

**Connection String Format:**
```bash
# Local development
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@localhost:5435/postgres

# Docker development
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres

# Production
PROVING_SERVICE_DATABASE_URL=postgresql://user:password@db.example.com:5432/proving_service
```

## Database Schema

### Fossil API Database Schema

#### `api_keys` Table

Stores API keys for authenticating requests to the Fossil API.

```sql
CREATE TABLE IF NOT EXISTS public.api_keys (
    id SERIAL PRIMARY KEY,
    key TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT api_keys_key_key UNIQUE (key)
);
```

**Columns:**
- `id`: Auto-incrementing primary key
- `key`: Unique API key string
- `name`: Descriptive name for the API key
- `created_at`: Timestamp when the key was created

#### `job_requests` Table

Tracks job requests and their processing status.

```sql
CREATE TABLE IF NOT EXISTS public.job_requests (
    job_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE,
    status VARCHAR(20) NOT NULL,
    result JSONB,
    vault_address TEXT,
    expected_timestamp BIGINT,
    l1_data JSONB,
    on_chain_confirmation JSONB,
    CONSTRAINT job_requests_pkey PRIMARY KEY (job_id),
    CONSTRAINT job_requests_status_check CHECK (
        status::TEXT = ANY (ARRAY['Completed'::TEXT, 'Pending'::TEXT, 'Failed'::TEXT])
    )
);

-- Performance indexes
CREATE INDEX idx_job_requests_pending_vault ON job_requests (status, vault_address)
WHERE status = 'Pending' AND vault_address IS NOT NULL;

CREATE INDEX idx_job_requests_timestamp ON job_requests (expected_timestamp)
WHERE expected_timestamp IS NOT NULL;
```

**Columns:**
- `job_id`: Unique identifier for the job (primary key)
- `created_at`: When the job was created
- `updated_at`: Last update timestamp
- `status`: Job status (Pending, Completed, Failed)
- `result`: JSON result data from processing
- `vault_address`: StarkNet vault contract address
- `expected_timestamp`: Expected completion timestamp
- `l1_data`: L1 pricing data (TWAP, max return, reserve price)
- `on_chain_confirmation`: Block number, transaction hash, event timestamp

**Indexes:**
- `idx_job_requests_pending_vault`: Optimizes lookup of pending jobs by vault
- `idx_job_requests_timestamp`: Optimizes timestamp-based queries

### Indexer Database Schema

#### `blockheaders` Table

Stores Ethereum L1 block header data for pricing calculations.

```sql
CREATE TABLE IF NOT EXISTS blockheaders (
    block_hash CHAR(66) UNIQUE,
    number BIGINT PRIMARY KEY,
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT NOT NULL,
    base_fee_per_gas VARCHAR(78),
    nonce VARCHAR(78) NOT NULL,
    transaction_root CHAR(66),
    receipts_root CHAR(66),
    state_root CHAR(66),
    parent_hash VARCHAR(66),
    miner VARCHAR(42),
    logs_bloom VARCHAR(1024),
    difficulty VARCHAR(78),
    totalDifficulty VARCHAR(78),
    sha3_uncles VARCHAR(66),
    timestamp VARCHAR(100),
    extra_data VARCHAR(1024),
    mix_hash VARCHAR(66),
    withdrawals_root VARCHAR(66),
    blob_gas_used VARCHAR(78),
    excess_blob_gas VARCHAR(78),
    parent_beacon_block_root VARCHAR(66),
    requests_hash TEXT
);
```

**Key Columns:**
- `number`: Block number (primary key)
- `block_hash`: Block hash (unique)
- `base_fee_per_gas`: Base fee for gas pricing (primary field used for TWAP calculations)
- `timestamp`: Block timestamp
- `gas_limit`, `gas_used`: Gas metrics

**Note:** This table is populated by the fossil-indexer service and is read-only for other services.

### Proving Service Database Schema

The Proving Service database uses the same `blockheaders` table structure as the Indexer database for local proof generation. It's initialized from `proving-service/tests/init.sql` for testing purposes.

## Running Migrations

### Fossil API Migrations

The Fossil API uses SQLx migrations located in `fossil-api/crates/db-access/migrations/`.

#### Manual Migration

```bash
# 1. Ensure database is running
docker compose -f docker-compose.local.yml up -d fossil_api_db

# 2. Run migrations
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  sqlx migrate run --source fossil-api/crates/db-access/migrations

# 3. Verify migration status
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  sqlx migrate info --source fossil-api/crates/db-access/migrations
```

#### Automatic Migration on Startup

The Fossil API server automatically runs migrations on startup via the `migrate()` method:

```rust
// In fossil-api/crates/db-access/src/lib.rs
pub async fn migrate(&self) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(&self.db_connection().pool)
        .await?;
    Ok(())
}
```

This is called during server initialization:

```rust
// Server startup
let db = OffchainProcessorDbConnection::from_env().await?;
db.migrate().await?;
```

#### Docker Container Migration

When using Docker, migrations are automatically run by the server container:

```bash
# The server container runs migrations on startup
docker compose -f docker-compose.local.yml up fossil-api
```

### Proving Service Migrations

The Proving Service currently uses a test initialization script rather than formal migrations:

```bash
# The init.sql script is automatically loaded by PostgreSQL on first startup
docker compose -f docker-compose.local.yml up -d proving_service_db

# The script is located at:
# proving-service/tests/init.sql
```

**Note:** The Proving Service does not currently use SQLx migrations. Schema changes are applied via the `init.sql` test script.

## Creating New Migrations

### For Fossil API

SQLx migrations are timestamped SQL files with `.up.sql` (apply) and `.down.sql` (rollback) versions.

#### Step 1: Create Migration Files

```bash
# Navigate to fossil-api directory
cd fossil-api

# Create a new migration
sqlx migrate add -r <migration_name> --source crates/db-access/migrations

# This creates two files:
# crates/db-access/migrations/YYYYMMDDHHMMSS_<migration_name>.up.sql
# crates/db-access/migrations/YYYYMMDDHHMMSS_<migration_name>.down.sql
```

**Naming Convention:**
- Use descriptive names: `add_user_table`, `add_index_to_jobs`, `alter_vault_fields`
- Timestamp format: `YYYYMMDDHHMMSS` (automatically added by sqlx)

#### Step 2: Write Migration SQL

**Example: Adding a new column**

`.up.sql`:
```sql
-- Add description column to job_requests
ALTER TABLE job_requests
ADD COLUMN description TEXT;

-- Add index for faster searches
CREATE INDEX idx_job_requests_description ON job_requests(description)
WHERE description IS NOT NULL;
```

`.down.sql`:
```sql
-- Remove index
DROP INDEX IF EXISTS idx_job_requests_description;

-- Remove column
ALTER TABLE job_requests
DROP COLUMN IF EXISTS description;
```

#### Step 3: Test the Migration

```bash
# 1. Start database
make dev-services

# 2. Run the migration
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  sqlx migrate run --source crates/db-access/migrations

# 3. Verify migration applied
psql postgresql://postgres:postgres@localhost:5434/postgres -c "\d job_requests"

# 4. Test rollback (optional)
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  sqlx migrate revert --source crates/db-access/migrations
```

#### Step 4: Update SQLx Offline Data

After modifying database queries or schema:

```bash
# 1. Ensure migrations are applied
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  sqlx migrate run --source crates/db-access/migrations

# 2. Regenerate query metadata
OFFCHAIN_PROCESSOR_DATABASE_URL="postgresql://postgres:postgres@localhost:5434/postgres" \
  cargo sqlx prepare --workspace

# 3. Commit the updated .sqlx directory
git add .sqlx
git commit -m "Update SQLx query metadata for <migration_name>"
```

**Why SQLx Offline Mode?**
- No database required during CI/CD builds
- Faster build times
- More reliable in restricted environments
- Query validation at compile time

### Migration Best Practices

1. **Always Create Both .up.sql and .down.sql**
   - Enables rollback in case of issues
   - Makes migrations reversible

2. **Use IF EXISTS / IF NOT EXISTS**
   ```sql
   CREATE TABLE IF NOT EXISTS users (...);
   DROP INDEX IF EXISTS idx_users_email;
   ```

3. **Test Migrations Locally**
   - Apply migration (`.up.sql`)
   - Verify schema changes
   - Test rollback (`.down.sql`)
   - Re-apply to ensure idempotency

4. **Keep Migrations Small**
   - One logical change per migration
   - Easier to review and debug
   - Safer rollbacks

5. **Add Indexes for Performance**
   - Consider query patterns
   - Use partial indexes with WHERE clauses
   - Index foreign keys

6. **Handle Data Migration Carefully**
   ```sql
   -- Good: Handle NULL values
   ALTER TABLE jobs ADD COLUMN priority INTEGER DEFAULT 0 NOT NULL;

   -- Better: Two-step migration for existing data
   ALTER TABLE jobs ADD COLUMN priority INTEGER;
   UPDATE jobs SET priority = 0 WHERE priority IS NULL;
   ALTER TABLE jobs ALTER COLUMN priority SET NOT NULL;
   ```

## Database Backups

### Local Development Backups

#### Backup Single Database

```bash
# Fossil API database
docker exec -t fossil-monorepo-fossil_api_db-1 pg_dump -U postgres -d postgres \
  > fossil_api_backup_$(date +%Y%m%d_%H%M%S).sql

# Proving Service database
docker exec -t fossil-monorepo-proving_service_db-1 pg_dump -U postgres -d postgres \
  > proving_service_backup_$(date +%Y%m%d_%H%M%S).sql

# Indexer database (if running locally)
docker exec -t fossil-monorepo-indexer_db-1 pg_dump -U postgres -d postgres \
  > indexer_backup_$(date +%Y%m%d_%H%M%S).sql
```

#### Backup All Databases

```bash
#!/bin/bash
# backup-all-dbs.sh

BACKUP_DIR="./db-backups/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

echo "Backing up all databases to $BACKUP_DIR..."

# Fossil API
docker exec -t fossil-monorepo-fossil_api_db-1 pg_dump -U postgres -d postgres \
  > "$BACKUP_DIR/fossil_api.sql"
echo "✅ Fossil API database backed up"

# Proving Service
docker exec -t fossil-monorepo-proving_service_db-1 pg_dump -U postgres -d postgres \
  > "$BACKUP_DIR/proving_service.sql"
echo "✅ Proving Service database backed up"

# Indexer (if exists)
if docker ps | grep -q indexer_db; then
    docker exec -t fossil-monorepo-indexer_db-1 pg_dump -U postgres -d postgres \
      > "$BACKUP_DIR/indexer.sql"
    echo "✅ Indexer database backed up"
fi

echo "✅ Backup complete: $BACKUP_DIR"
```

Make it executable:
```bash
chmod +x backup-all-dbs.sh
./backup-all-dbs.sh
```

#### Compressed Backups

```bash
# Fossil API (compressed)
docker exec -t fossil-monorepo-fossil_api_db-1 pg_dump -U postgres -d postgres \
  | gzip > fossil_api_backup_$(date +%Y%m%d_%H%M%S).sql.gz

# Proving Service (compressed)
docker exec -t fossil-monorepo-proving_service_db-1 pg_dump -U postgres -d postgres \
  | gzip > proving_service_backup_$(date +%Y%m%d_%H%M%S).sql.gz
```

### Restoring Backups

#### Restore from SQL Backup

```bash
# Fossil API
docker exec -i fossil-monorepo-fossil_api_db-1 psql -U postgres -d postgres \
  < fossil_api_backup_20250106_143022.sql

# Proving Service
docker exec -i fossil-monorepo-proving_service_db-1 psql -U postgres -d postgres \
  < proving_service_backup_20250106_143022.sql
```

#### Restore from Compressed Backup

```bash
# Fossil API (compressed)
gunzip < fossil_api_backup_20250106_143022.sql.gz | \
  docker exec -i fossil-monorepo-fossil_api_db-1 psql -U postgres -d postgres

# Proving Service (compressed)
gunzip < proving_service_backup_20250106_143022.sql.gz | \
  docker exec -i fossil-monorepo-proving_service_db-1 psql -U postgres -d postgres
```

#### Clean Restore (Drop and Recreate)

```bash
# Drop and recreate database before restoring
docker exec -i fossil-monorepo-fossil_api_db-1 psql -U postgres << EOF
DROP DATABASE IF EXISTS postgres;
CREATE DATABASE postgres;
EOF

# Restore backup
docker exec -i fossil-monorepo-fossil_api_db-1 psql -U postgres -d postgres \
  < fossil_api_backup_20250106_143022.sql
```

### Production Backups

For production environments, use managed database backup solutions:

#### AWS RDS Automated Backups

```bash
# Create manual snapshot
aws rds create-db-snapshot \
  --db-instance-identifier fossil-api-prod \
  --db-snapshot-identifier fossil-api-manual-$(date +%Y%m%d-%H%M%S)

# Restore from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier fossil-api-restored \
  --db-snapshot-identifier fossil-api-manual-20250106-143022
```

#### Manual Production Backup

```bash
# Connect to production via bastion/VPN
pg_dump -h db.example.com -U dbuser -d fossil_api \
  | gzip > fossil_api_prod_backup_$(date +%Y%m%d_%H%M%S).sql.gz

# Store in S3
aws s3 cp fossil_api_prod_backup_20250106_143022.sql.gz \
  s3://fossil-db-backups/$(date +%Y/%m/%d)/
```

### Backup Best Practices

1. **Regular Backup Schedule**
   - Development: Before major migrations
   - Staging: Daily automated backups
   - Production: Continuous backups with point-in-time recovery

2. **Test Restore Procedures**
   ```bash
   # Periodically test restore process
   ./backup-all-dbs.sh
   # Restore to a test database and verify
   ```

3. **Backup Retention**
   - Keep last 7 daily backups
   - Keep last 4 weekly backups
   - Keep last 12 monthly backups
   - Archive important backups separately

4. **Secure Backup Storage**
   - Encrypt backups at rest
   - Use versioned storage (S3 with versioning)
   - Limit access with IAM policies
   - Test backup integrity regularly

## Connection Management

### Connection Pool Configuration

Both Fossil API and Proving Service use SQLx connection pooling with configurable parameters.

#### Current Configuration

```rust
// fossil-api/crates/db-access/src/lib.rs
// proving-service/crates/db/src/lib.rs

let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(database_url)
    .await?;
```

**Default Settings:**
- `max_connections`: 5 concurrent connections
- Connection timeout: 30 seconds (SQLx default)
- Idle timeout: 10 minutes (SQLx default)
- Acquire timeout: 30 seconds (SQLx default)

#### Advanced Pool Configuration

For production or high-concurrency scenarios:

```rust
use std::time::Duration;
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(20)                          // Increase for high concurrency
    .min_connections(5)                           // Maintain minimum connections
    .max_lifetime(Duration::from_secs(30 * 60))   // 30 minutes max connection lifetime
    .idle_timeout(Duration::from_secs(10 * 60))   // 10 minutes idle timeout
    .acquire_timeout(Duration::from_secs(30))     // 30 seconds acquire timeout
    .test_before_acquire(true)                    // Validate connections before use
    .connect(database_url)
    .await?;
```

**Configuration Guidelines:**

| Setting | Development | Staging | Production |
|---------|------------|---------|------------|
| `max_connections` | 5 | 10-20 | 20-50 |
| `min_connections` | 1 | 5 | 10 |
| `max_lifetime` | 30 min | 30 min | 15-30 min |
| `idle_timeout` | 10 min | 10 min | 5-10 min |
| `acquire_timeout` | 30s | 30s | 30-60s |

### Connection String Parameters

SQLx supports additional connection parameters via URL query strings:

```bash
# Basic connection
postgresql://user:password@host:port/database

# With SSL (production)
postgresql://user:password@host:port/database?sslmode=require

# With connection pool settings
postgresql://user:password@host:port/database?sslmode=require&connect_timeout=10&application_name=fossil_api

# Read-only connection (for Indexer database)
postgresql://user:password@host:port/database?sslmode=require&default_transaction_read_only=true
```

**Common Parameters:**
- `sslmode`: `disable`, `allow`, `prefer`, `require`, `verify-ca`, `verify-full`
- `connect_timeout`: Connection timeout in seconds
- `application_name`: Name shown in database logs
- `statement_timeout`: Maximum query execution time (milliseconds)
- `default_transaction_read_only`: Force read-only transactions

### Connection Monitoring

#### Check Active Connections

```sql
-- View active connections by database
SELECT
    datname,
    count(*) as connections,
    max_conn,
    count(*) * 100.0 / max_conn as percent_used
FROM pg_stat_activity
JOIN (SELECT setting::int as max_conn FROM pg_settings WHERE name = 'max_connections') mc ON true
WHERE datname = 'postgres'
GROUP BY datname, max_conn;

-- View connections by application
SELECT
    application_name,
    state,
    count(*) as count
FROM pg_stat_activity
WHERE datname = 'postgres'
GROUP BY application_name, state
ORDER BY count DESC;
```

#### Kill Long-Running Queries

```sql
-- Find long-running queries
SELECT
    pid,
    now() - pg_stat_activity.query_start AS duration,
    state,
    query
FROM pg_stat_activity
WHERE state != 'idle'
  AND now() - pg_stat_activity.query_start > interval '5 minutes'
ORDER BY duration DESC;

-- Kill a specific query
SELECT pg_terminate_backend(12345);  -- Replace with actual PID

-- Kill all idle connections
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle'
  AND now() - state_change > interval '30 minutes';
```

### Connection Troubleshooting

#### Too Many Connections

**Error:** `FATAL: sorry, too many clients already`

**Solutions:**

1. **Increase max_connections**
   ```sql
   -- Check current limit
   SHOW max_connections;

   -- Increase limit (requires restart)
   ALTER SYSTEM SET max_connections = 100;
   -- Then restart PostgreSQL
   ```

2. **Reduce connection pool size**
   ```rust
   // Decrease max_connections in application code
   .max_connections(3)  // Reduced from 5
   ```

3. **Use connection pooling middleware**
   - Consider PgBouncer for production
   - Reduces database connection overhead

#### Connection Timeout

**Error:** `connection timed out` or `could not connect to server`

**Solutions:**

1. **Increase acquire timeout**
   ```rust
   .acquire_timeout(Duration::from_secs(60))
   ```

2. **Check database is running**
   ```bash
   docker ps | grep postgres

   # Test connection directly
   psql postgresql://postgres:postgres@localhost:5434/postgres
   ```

3. **Verify network/firewall settings**
   ```bash
   # Check port is accessible
   telnet localhost 5434

   # Check Docker network
   docker network inspect fossil-monorepo_local-network
   ```

## Common Operations

### Database Access

#### Using psql CLI

```bash
# Fossil API database
psql postgresql://postgres:postgres@localhost:5434/postgres

# Proving Service database
psql postgresql://postgres:postgres@localhost:5435/postgres

# Indexer database
psql postgresql://postgres:postgres@localhost:5433/postgres

# Via Docker exec
docker exec -it fossil-monorepo-fossil_api_db-1 psql -U postgres -d postgres
```

#### Common psql Commands

```sql
-- List all databases
\l

-- List all tables
\dt

-- Describe table structure
\d job_requests
\d+ job_requests  -- With additional details

-- List all indexes
\di

-- View table sizes
\dt+

-- Execute SQL from file
\i /path/to/script.sql

-- Toggle query timing
\timing

-- Quit
\q
```

### Useful SQL Queries

#### Job Request Queries

```sql
-- Count jobs by status
SELECT status, COUNT(*)
FROM job_requests
GROUP BY status;

-- Find recent pending jobs
SELECT job_id, vault_address, expected_timestamp, created_at
FROM job_requests
WHERE status = 'Pending'
ORDER BY created_at DESC
LIMIT 10;

-- Find failed jobs with error details
SELECT job_id, created_at, result->>'error' as error_message
FROM job_requests
WHERE status = 'Failed'
ORDER BY created_at DESC;

-- Job completion rate
SELECT
    DATE(created_at) as date,
    COUNT(*) FILTER (WHERE status = 'Completed') * 100.0 / COUNT(*) as completion_rate,
    COUNT(*) as total_jobs
FROM job_requests
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY DATE(created_at)
ORDER BY date DESC;

-- Average job processing time
SELECT
    AVG(updated_at - created_at) as avg_duration,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY updated_at - created_at) as median_duration
FROM job_requests
WHERE status = 'Completed'
  AND updated_at IS NOT NULL;
```

#### API Key Queries

```sql
-- List all API keys
SELECT id, name, created_at FROM api_keys ORDER BY created_at DESC;

-- Find API key by name
SELECT * FROM api_keys WHERE name LIKE '%test%';

-- Create new API key
INSERT INTO api_keys (key, name)
VALUES ('sk_test_' || gen_random_uuid()::text, 'Test API Key')
RETURNING *;

-- Delete old API keys
DELETE FROM api_keys
WHERE created_at < NOW() - INTERVAL '1 year'
  AND name LIKE '%test%';
```

#### Block Header Queries (Indexer/Proving Service)

```sql
-- Get block headers in time range
SELECT
    number,
    block_hash,
    base_fee_per_gas,
    TO_TIMESTAMP(CAST(timestamp AS BIGINT)) as block_time
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN 1743249000 AND 1743249120
ORDER BY number ASC;

-- Calculate average base fee over time window
SELECT
    AVG(CAST('x' || LTRIM(base_fee_per_gas, '0x') AS bit(256))::bigint) as avg_base_fee,
    COUNT(*) as block_count
FROM blockheaders
WHERE CAST(timestamp AS BIGINT) BETWEEN 1743249000 AND 1743249120;

-- Find blocks with high gas usage
SELECT
    number,
    block_hash,
    gas_used,
    gas_limit,
    (gas_used::float / gas_limit::float) * 100 as utilization_percent
FROM blockheaders
WHERE (gas_used::float / gas_limit::float) > 0.9
ORDER BY number DESC
LIMIT 20;
```

### Database Maintenance

#### Vacuum and Analyze

```sql
-- Vacuum to reclaim storage
VACUUM job_requests;

-- Vacuum with analyze (recommended)
VACUUM ANALYZE job_requests;

-- Full vacuum (locks table, use during maintenance window)
VACUUM FULL job_requests;

-- Analyze only (update statistics)
ANALYZE job_requests;

-- Auto vacuum settings
SHOW autovacuum;
```

#### Rebuild Indexes

```sql
-- Rebuild index to reduce bloat
REINDEX INDEX idx_job_requests_pending_vault;

-- Rebuild all indexes on table
REINDEX TABLE job_requests;

-- Rebuild entire database (use with caution)
REINDEX DATABASE postgres;
```

#### Check Database Size

```sql
-- Database size
SELECT
    pg_database.datname,
    pg_size_pretty(pg_database_size(pg_database.datname)) AS size
FROM pg_database
ORDER BY pg_database_size(pg_database.datname) DESC;

-- Table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Index sizes
SELECT
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(schemaname||'.'||indexname)) AS size
FROM pg_indexes
WHERE schemaname = 'public'
ORDER BY pg_relation_size(schemaname||'.'||indexname) DESC;
```

## Troubleshooting

### Database Connection Errors

#### Problem: `connection refused`

**Symptoms:**
```
Error: error connecting to server: Connection refused (os error 111)
```

**Solutions:**

```bash
# 1. Check if PostgreSQL container is running
docker ps | grep postgres

# 2. Verify port mapping
docker port fossil-monorepo-fossil_api_db-1

# 3. Check logs for errors
docker logs fossil-monorepo-fossil_api_db-1

# 4. Restart database container
docker restart fossil-monorepo-fossil_api_db-1

# 5. Verify connection string
echo $OFFCHAIN_PROCESSOR_DATABASE_URL

# 6. Test connection directly
psql postgresql://postgres:postgres@localhost:5434/postgres
```

#### Problem: `password authentication failed`

**Symptoms:**
```
FATAL: password authentication failed for user "postgres"
```

**Solutions:**

```bash
# 1. Verify credentials in connection string
echo $OFFCHAIN_PROCESSOR_DATABASE_URL

# 2. Check Docker environment variables
docker inspect fossil-monorepo-fossil_api_db-1 | grep -A 5 Env

# 3. Reset database (development only!)
docker compose -f docker-compose.local.yml down -v
docker compose -f docker-compose.local.yml up -d fossil_api_db

# 4. Verify PostgreSQL is accepting connections
docker exec fossil-monorepo-fossil_api_db-1 psql -U postgres -c "SELECT 1"
```

#### Problem: `database does not exist`

**Symptoms:**
```
FATAL: database "mydb" does not exist
```

**Solutions:**

```bash
# 1. List available databases
docker exec fossil-monorepo-fossil_api_db-1 psql -U postgres -c "\l"

# 2. Create database if missing
docker exec fossil-monorepo-fossil_api_db-1 psql -U postgres -c "CREATE DATABASE postgres"

# 3. Verify connection string uses correct database name
echo $OFFCHAIN_PROCESSOR_DATABASE_URL
```

### Migration Errors

#### Problem: Migration already applied

**Symptoms:**
```
error: migration 20241024044528 was previously applied but has been modified
```

**Solutions:**

```bash
# 1. Check migration status
sqlx migrate info --source crates/db-access/migrations

# 2. Revert last migration
sqlx migrate revert --source crates/db-access/migrations

# 3. Re-apply migrations
sqlx migrate run --source crates/db-access/migrations

# 4. For local development only - reset migration table
psql postgresql://postgres:postgres@localhost:5434/postgres << EOF
DROP TABLE IF EXISTS _sqlx_migrations;
EOF
```

#### Problem: Migration conflict

**Symptoms:**
```
error: migration 20250106143022 conflicts with existing schema
```

**Solutions:**

```bash
# 1. Backup database first
docker exec -t fossil-monorepo-fossil_api_db-1 pg_dump -U postgres -d postgres > backup.sql

# 2. Check current schema
psql postgresql://postgres:postgres@localhost:5434/postgres -c "\d job_requests"

# 3. Option A: Modify migration to handle existing schema
# Edit migration to use IF NOT EXISTS / IF EXISTS

# 4. Option B: Revert to clean state (development only!)
docker compose -f docker-compose.local.yml down -v
docker compose -f docker-compose.local.yml up -d fossil_api_db
sqlx migrate run --source crates/db-access/migrations
```

### Performance Issues

#### Problem: Slow queries

**Symptoms:**
- API requests timing out
- High database CPU usage
- Long-running queries

**Diagnosis:**

```sql
-- Find slow queries
SELECT
    pid,
    now() - pg_stat_activity.query_start AS duration,
    state,
    query
FROM pg_stat_activity
WHERE state != 'idle'
  AND now() - pg_stat_activity.query_start > interval '1 second'
ORDER BY duration DESC;

-- Check table statistics
SELECT * FROM pg_stat_user_tables WHERE schemaname = 'public';

-- Check index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan ASC;
```

**Solutions:**

```sql
-- 1. Add missing indexes
CREATE INDEX idx_job_requests_status ON job_requests(status);
CREATE INDEX idx_job_requests_created_at ON job_requests(created_at DESC);

-- 2. Update table statistics
ANALYZE job_requests;

-- 3. Use EXPLAIN to analyze query plans
EXPLAIN ANALYZE
SELECT * FROM job_requests WHERE status = 'Pending' ORDER BY created_at DESC;

-- 4. Consider partial indexes for common queries
CREATE INDEX idx_pending_jobs ON job_requests(created_at DESC)
WHERE status = 'Pending';
```

#### Problem: Table bloat

**Symptoms:**
- Large table sizes
- Slow queries despite indexes
- High disk usage

**Diagnosis:**

```sql
-- Check table bloat
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) as table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) -
                   pg_relation_size(schemaname||'.'||tablename)) as indexes_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

**Solutions:**

```sql
-- 1. Vacuum to reclaim space
VACUUM FULL job_requests;

-- 2. Rebuild indexes
REINDEX TABLE job_requests;

-- 3. Enable autovacuum if disabled
ALTER TABLE job_requests SET (autovacuum_enabled = true);

-- 4. Delete old data
DELETE FROM job_requests
WHERE status = 'Completed'
  AND created_at < NOW() - INTERVAL '90 days';
```

### Data Integrity Issues

#### Problem: Orphaned records

**Symptoms:**
- Foreign key violations (if using FKs)
- Jobs referencing non-existent vaults

**Diagnosis:**

```sql
-- Find jobs with invalid vault addresses (example)
SELECT job_id, vault_address, status
FROM job_requests
WHERE vault_address IS NOT NULL
  AND vault_address NOT LIKE '0x%';

-- Check for NULL values where not expected
SELECT COUNT(*)
FROM job_requests
WHERE expected_timestamp IS NULL AND vault_address IS NOT NULL;
```

**Solutions:**

```sql
-- Clean up invalid records
DELETE FROM job_requests
WHERE vault_address IS NOT NULL
  AND vault_address NOT LIKE '0x%';

-- Add constraints to prevent future issues
ALTER TABLE job_requests
ADD CONSTRAINT valid_vault_address
CHECK (vault_address IS NULL OR vault_address LIKE '0x%');
```

### Docker Volume Issues

#### Problem: Data not persisting

**Symptoms:**
- Database resets on container restart
- Migrations run every startup

**Solutions:**

```bash
# 1. Check if volumes exist
docker volume ls | grep fossil

# 2. Inspect volume
docker volume inspect fossil-monorepo_fossil_api_local_data

# 3. Verify volume mount
docker inspect fossil-monorepo-fossil_api_db-1 | grep -A 10 Mounts

# 4. If volume is corrupted, recreate (data loss!)
docker compose -f docker-compose.local.yml down -v
docker volume rm fossil-monorepo_fossil_api_local_data
docker compose -f docker-compose.local.yml up -d fossil_api_db
```

#### Problem: Permission errors in volume

**Symptoms:**
```
initdb: could not create directory "/var/lib/postgresql/data": Permission denied
```

**Solutions:**

```bash
# 1. Check volume permissions
docker exec fossil-monorepo-fossil_api_db-1 ls -la /var/lib/postgresql/

# 2. Fix ownership (if needed)
docker exec fossil-monorepo-fossil_api_db-1 chown -R postgres:postgres /var/lib/postgresql/data

# 3. Recreate with correct permissions
docker compose -f docker-compose.local.yml down
docker volume rm fossil-monorepo_fossil_api_local_data
docker compose -f docker-compose.local.yml up -d fossil_api_db
```

## Next Steps

- [Environment Setup](environment-setup.md) - Configure database connection strings
- [Running Services](running-services.md) - Start databases with services
- [Deployment Guide](deployment.md) - Production database setup and RDS configuration
- [Testing Guide](../getting-started/testing.md) - Database setup for tests
- [Fossil API Architecture](../architecture/fossil-api.md) - Database layer details
- [Proving Service Architecture](../architecture/proving-service.md) - Database usage patterns
