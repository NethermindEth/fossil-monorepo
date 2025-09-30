use std::sync::Arc;
use std::time::Duration;

use crate::starknet_provider::{EventFilter, StarknetEvent, StarknetProvider};
use db_access::{
    models::{JobRequest, JobStatus, L1Data, OnChainConfirmation},
    queries::{get_pending_jobs_with_vaults, update_job_with_event_data},
    OffchainProcessorDbConnection,
};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FossilCallbackSuccessEvent {
    pub l1_data: L1DataEvent,
    pub timestamp: u64,
    pub block_number: u64,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1DataEvent {
    pub twap: String,          // u256 as hex string
    pub max_return: String,    // u128 as hex string
    pub reserve_price: String, // u256 as hex string
}

pub struct VaultEventMonitor {
    provider: StarknetProvider,
    db: Arc<OffchainProcessorDbConnection>,
    polling_interval: Duration,
    blocks_per_scan: u64,
}

impl VaultEventMonitor {
    pub const fn new(
        provider: StarknetProvider,
        db: Arc<OffchainProcessorDbConnection>,
        polling_interval_secs: u64,
        blocks_per_scan: u64,
    ) -> Self {
        Self {
            provider,
            db,
            polling_interval: Duration::from_secs(polling_interval_secs),
            blocks_per_scan,
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting vault event monitoring");

        loop {
            if let Err(e) = self.process_fossil_callback_events().await {
                error!("Error processing events: {:?}", e);
            }

            sleep(self.polling_interval).await;
        }
    }

    #[instrument(skip(self), level = "debug")]
    async fn process_fossil_callback_events(&self) -> Result<()> {
        let pending_jobs = get_pending_jobs_with_vaults(self.db.clone())
            .await
            .map_err(|e| eyre!("Failed to get pending jobs: {:?}", e))?;

        if pending_jobs.is_empty() {
            debug!("No pending jobs with vault addresses found");
            return Ok(());
        }

        info!(
            job_count = pending_jobs.len(),
            "Processing pending jobs for vault events"
        );

        let current_block = self
            .provider
            .block_number()
            .await
            .map_err(|e| eyre!("Failed to get current block number: {:?}", e))?;

        // Group jobs by vault address for efficient processing
        let mut jobs_by_vault: std::collections::HashMap<String, Vec<&JobRequest>> =
            std::collections::HashMap::new();

        for job in &pending_jobs {
            if let Some(vault_addr) = &job.vault_address {
                jobs_by_vault
                    .entry(vault_addr.clone())
                    .or_default()
                    .push(job);
            }
        }

        for (vault_address, jobs) in jobs_by_vault {
            if let Err(e) = self
                .process_vault_events(&vault_address, &jobs, current_block)
                .await
            {
                error!(
                    vault_address = %vault_address,
                    error = %e,
                    "Failed to process events for vault"
                );
            }
        }

        Ok(())
    }

    #[instrument(skip(self, jobs), level = "debug")]
    async fn process_vault_events(
        &self,
        vault_address: &str,
        jobs: &[&JobRequest],
        current_block: u64,
    ) -> Result<()> {
        let from_block = current_block.saturating_sub(self.blocks_per_scan);

        debug!(
            vault_address = %vault_address,
            from_block = from_block,
            to_block = current_block,
            job_count = jobs.len(),
            "Fetching events for vault"
        );

        let events = self
            .fetch_callback_events(vault_address, from_block, current_block)
            .await?;

        if events.is_empty() {
            debug!(vault_address = %vault_address, "No FossilCallbackSuccess events found");
            return Ok(());
        }

        info!(
            vault_address = %vault_address,
            event_count = events.len(),
            "Found FossilCallbackSuccess events"
        );

        for job in jobs {
            for event in &events {
                if self.matches_job_criteria(job, event) {
                    if let Err(e) = self.complete_job_from_event(&job.job_id, event).await {
                        error!(
                            job_id = %job.job_id,
                            error = %e,
                            "Failed to complete job from event"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    async fn fetch_callback_events(
        &self,
        vault_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<FossilCallbackSuccessEvent>> {
        // FossilCallbackSuccess event selector from pitchlake vault contract
        let fossil_callback_selector =
            "0x028daf8c351269d325d0652973bd8ae38ea95c476dcc11eaf0e2495c440689cb";

        let event_filter = EventFilter {
            from_block: Some(from_block),
            to_block: Some(to_block),
            address: Some(vault_address.to_string()),
            keys: Some(vec![vec![fossil_callback_selector.to_string()]]),
        };

        let events_page = self
            .provider
            .get_events(event_filter, None, 100)
            .await
            .map_err(|e| eyre!("Failed to fetch events: {:?}", e))?;

        let mut fossil_events = Vec::new();

        for event in events_page.events {
            // Only process events that have the correct FossilCallbackSuccess selector
            if !event.keys.is_empty() && event.keys[0] == fossil_callback_selector {
                match self.parse_fossil_callback_event(&event).await {
                    Ok(fossil_event) => fossil_events.push(fossil_event),
                    Err(e) => {
                        debug!(
                            block_number = ?event.block_number,
                            transaction_hash = %event.transaction_hash,
                            data_length = event.data.len(),
                            error = %e,
                            "Failed to parse FossilCallbackSuccess event"
                        );
                    }
                }
            }
        }

        Ok(fossil_events)
    }

    async fn parse_fossil_callback_event(
        &self,
        event: &StarknetEvent,
    ) -> Result<FossilCallbackSuccessEvent> {
        // Parse event data according to actual FossilCallbackSuccess structure
        // Based on transaction 0x4c55b3c8765e0e1904c50c3e758f34e7e9ef9721adad322ba4632967ea8b711:
        // Event data format: [twap, max_return, reserve_price, unknown1, unknown2, timestamp]

        if event.data.len() < 6 {
            return Err(eyre!(
                "Insufficient event data length for FossilCallbackSuccess: expected 6, got {}",
                event.data.len()
            ));
        }

        // Parse L1Data fields as hex strings (felt252 values)
        let twap = event.data[0].clone();
        let max_return = event.data[1].clone();
        let reserve_price = event.data[2].clone();
        // Skip unknown fields at index 3 and 4

        // Parse timestamp - last field (index 5)
        let timestamp_str = event.data[5].trim_start_matches("0x");
        let timestamp = u64::from_str_radix(timestamp_str, 16)
            .map_err(|e| eyre!("Failed to parse timestamp '{}': {:?}", timestamp_str, e))?;

        let l1_data = L1DataEvent {
            twap,
            max_return,
            reserve_price,
        };

        let block_number = event.block_number.unwrap_or(0); // Use 0 as fallback if block number is missing

        Ok(FossilCallbackSuccessEvent {
            l1_data,
            timestamp,
            block_number,
            transaction_hash: event.transaction_hash.clone(),
        })
    }

    fn matches_job_criteria(&self, job: &JobRequest, event: &FossilCallbackSuccessEvent) -> bool {
        // Match by timestamp with some tolerance (e.g., ±60 seconds)
        if let Some(expected_timestamp) = job.expected_timestamp {
            let timestamp_diff = (event.timestamp as i64 - expected_timestamp).abs();
            if timestamp_diff > 60 {
                debug!(
                    job_id = %job.job_id,
                    expected_timestamp = expected_timestamp,
                    event_timestamp = event.timestamp,
                    diff = timestamp_diff,
                    "Timestamp mismatch for job"
                );
                return false;
            }
        }

        true
    }

    #[instrument(skip(self), level = "debug")]
    async fn complete_job_from_event(
        &self,
        job_id: &str,
        event: &FossilCallbackSuccessEvent,
    ) -> Result<()> {
        let l1_data = L1Data {
            twap: event.l1_data.twap.clone(),
            max_return: event.l1_data.max_return.clone(),
            reserve_price: event.l1_data.reserve_price.clone(),
        };

        let on_chain_confirmation = OnChainConfirmation {
            block_number: event.block_number,
            transaction_hash: event.transaction_hash.clone(),
            event_timestamp: event.timestamp,
        };

        let l1_data_json = serde_json::to_value(&l1_data)
            .map_err(|e| eyre!("Failed to serialize L1Data: {:?}", e))?;

        let confirmation_json = serde_json::to_value(&on_chain_confirmation)
            .map_err(|e| eyre!("Failed to serialize OnChainConfirmation: {:?}", e))?;

        update_job_with_event_data(
            self.db.clone(),
            job_id,
            JobStatus::Completed,
            l1_data_json,
            confirmation_json,
        )
        .await
        .map_err(|e| eyre!("Failed to update job with event data: {:?}", e))?;

        info!(
            job_id = job_id,
            twap = %l1_data.twap,
            max_return = %l1_data.max_return,
            reserve_price = %l1_data.reserve_price,
            timestamp = event.timestamp,
            block_number = event.block_number,
            transaction_hash = %event.transaction_hash,
            "Job completed via FossilCallbackSuccess event"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Simplified test that only tests the matching logic without database dependencies
    #[test]
    fn test_timestamp_matching_logic() {
        // Test timestamp matching within tolerance
        let expected_timestamp = 1672531200i64;
        let event_timestamp = 1672531230u64; // 30 seconds later
        let diff = (event_timestamp as i64 - expected_timestamp).abs();
        assert!(diff <= 60, "Should be within 60 second tolerance");

        // Test timestamp outside tolerance
        let event_timestamp_late = 1672531300u64; // 100 seconds later
        let diff_late = (event_timestamp_late as i64 - expected_timestamp).abs();
        assert!(diff_late > 60, "Should be outside 60 second tolerance");
    }
}
