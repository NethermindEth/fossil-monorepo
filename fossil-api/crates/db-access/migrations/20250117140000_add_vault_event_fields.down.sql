-- Remove vault address and event tracking fields from job_requests table
DROP INDEX IF EXISTS idx_job_requests_pending_vault;
DROP INDEX IF EXISTS idx_job_requests_timestamp;

ALTER TABLE job_requests
DROP COLUMN IF EXISTS vault_address,
DROP COLUMN IF EXISTS expected_timestamp,
DROP COLUMN IF EXISTS l1_data,
DROP COLUMN IF EXISTS on_chain_confirmation;