-- Add updated_at column to job_requests table
ALTER TABLE job_requests ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITHOUT TIME ZONE;