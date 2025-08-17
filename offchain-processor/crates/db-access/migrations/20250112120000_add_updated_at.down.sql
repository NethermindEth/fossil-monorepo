-- Remove updated_at column from job_requests table
ALTER TABLE job_requests DROP COLUMN IF EXISTS updated_at;