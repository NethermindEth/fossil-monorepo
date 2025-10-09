-- Add vault address and event tracking fields to job_requests table
ALTER TABLE job_requests
ADD COLUMN vault_address TEXT,
ADD COLUMN expected_timestamp BIGINT,
ADD COLUMN l1_data JSONB,
ADD COLUMN on_chain_confirmation JSONB;

-- Create index for efficient lookup of pending jobs with vault addresses
CREATE INDEX idx_job_requests_pending_vault ON job_requests (status, vault_address)
WHERE status = 'Pending' AND vault_address IS NOT NULL;

-- Create index for timestamp-based lookups
CREATE INDEX idx_job_requests_timestamp ON job_requests (expected_timestamp)
WHERE expected_timestamp IS NOT NULL;