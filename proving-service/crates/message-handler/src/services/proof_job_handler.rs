use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

use crate::queue::message_queue::Queue;
use crate::{
    proof_composition::{ProofProvider, ProofTimestampRanges},
    services::jobs::ProofGenerated,
};
use eyre::{Result, eyre};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use super::jobs::Job;

// Track job processing state and failure count
#[derive(Debug, Clone)]
struct JobProcessingState {
    failure_count: u32,
    last_failure_time: std::time::Instant,
}

// Updated ProofJobHandler to use trait object for ProofProvider
pub struct ProofJobHandler<Q: Queue + Send + Sync + 'static> {
    queue: Arc<Q>,
    terminator: Arc<AtomicBool>,
    proof_provider: Arc<dyn ProofProvider + Send + Sync>,
    proof_generation_timeout: Duration,
    // Track currently processing job IDs to prevent duplicate processing
    processing_jobs: Arc<Mutex<HashSet<String>>>,
    // Track job failure counts to help decide when to forcibly delete
    job_failures: Arc<Mutex<HashMap<String, JobProcessingState>>>,
    // Maximum number of failures before forcing deletion
    max_failures: u32,
}

// Implementation for the updated ProofJobHandler
impl<Q> ProofJobHandler<Q>
where
    Q: Queue + Send + Sync + 'static,
{
    pub fn new(
        queue: Arc<Q>,
        terminator: Arc<AtomicBool>,
        proof_provider: Arc<dyn ProofProvider + Send + Sync>,
        proof_generation_timeout: Duration,
    ) -> Self {
        Self {
            queue,
            terminator,
            proof_provider,
            proof_generation_timeout,
            processing_jobs: Arc::new(Mutex::new(HashSet::new())),
            job_failures: Arc::new(Mutex::new(HashMap::new())),
            max_failures: 3, // Allow up to 3 failures before forcibly deleting
        }
    }

    pub async fn receive_job(&self) -> Result<()> {
        // Check if proof provider is disabled at startup
        if self.proof_provider.is_disabled() {
            info!("Proof generation is disabled, jobs will be acknowledged without processing");
        }

        // Create a join set to keep track of all the jobs;
        let mut join_set = JoinSet::new();
        info!("Starting to poll for messages from queue");
        let mut poll_counter = 0;

        while !self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
            poll_counter += 1;
            if poll_counter % 10 == 0 {
                debug!("Polling for messages (count: {})", poll_counter);
            }

            let messages = match self.queue.receive_messages().await {
                Ok(messages) => {
                    if !messages.is_empty() {
                        info!("Received {} messages from queue", messages.len());
                    }
                    messages
                }
                Err(e) => {
                    warn!("Error receiving messages from queue: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            if messages.is_empty() {
                // No messages, sleep briefly to avoid tight loop
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }

            for message in messages {
                // Log the raw message for debugging
                debug!("Received message body: {}", message.body);

                let job: Job = match serde_json::from_str(&message.body) {
                    Ok(job) => {
                        debug!("Successfully parsed Job variant");
                        job
                    }
                    Err(e) => {
                        warn!("Error parsing job: {}. Message body: {}", e, message.body);
                        // Delete invalid messages to prevent them from being processed repeatedly
                        if let Err(e) = self.queue.delete_message(&message).await {
                            error!("Error deleting invalid message from queue: {}", e);
                        } else {
                            info!("Deleted invalid message from queue");
                        }
                        continue;
                    }
                };

                // Only handle RequestProof jobs
                let job = if let Job::RequestProof(job) = job {
                    job
                } else {
                    // Delete non-RequestProof messages
                    if let Err(e) = self.queue.delete_message(&message).await {
                        error!("Error deleting non-RequestProof message from queue: {}", e);
                    } else {
                        info!("Deleted non-RequestProof message from queue");
                    }
                    continue;
                };

                // Check if the job is already being processed
                {
                    let mut processing_jobs = self.processing_jobs.lock().await;
                    if processing_jobs.contains(&job.job_id) {
                        info!("Job ID {} is already being processed, skipping", job.job_id);
                        continue;
                    }

                    // Mark this job as being processed
                    processing_jobs.insert(job.job_id.clone());
                }

                // Check if proof provider is disabled
                if self.proof_provider.is_disabled() {
                    info!(
                        "Proof generation is disabled. Acknowledging job without processing: {:?}",
                        job
                    );
                    // Delete the message from the queue to acknowledge it
                    if let Err(e) = self.queue.delete_message(&message).await {
                        error!("Error deleting disabled proof message from queue: {}", e);
                    }
                    continue;
                }

                // Clone all necessary variables for the task
                let message_clone = message.clone();
                let queue_clone = self.queue.clone();
                let proof_provider = self.proof_provider.clone();
                let timeout_duration = self.proof_generation_timeout;
                let processing_jobs = self.processing_jobs.clone();
                let job_failures = self.job_failures.clone();
                let max_failures = self.max_failures;

                join_set.spawn(async move {
                    info!("Starting processing for job ID: {}", job.job_id);
                    debug!("Received & processing job: {:?}", job);

                    // Check if this job has failed too many times
                    let should_delete_after_processing = {
                        let mut failures = job_failures.lock().await;
                        let failure_entry = failures.entry(job.job_id.clone())
                            .or_insert_with(|| JobProcessingState {
                                failure_count: 0,
                                last_failure_time: std::time::Instant::now(),
                            });
                        let should_delete = failure_entry.failure_count >= max_failures;
                        if should_delete {
                            info!("Job ID {} has failed {} times, will delete after processing", job.job_id, failure_entry.failure_count);
                        }
                        should_delete
                    };

                    // Create ProofTimestampRanges from the job
                    let timestamp_ranges = create_timestamp_ranges(&job);

                    // Check if this job has all the required timestamp ranges
                    let has_all_components = job.twap_start_timestamp.is_some() &&
                                         job.reserve_price_start_timestamp.is_some() &&
                                         job.max_return_start_timestamp.is_some();
                    if has_all_components {
                        info!("Processing job ID: {} with all three components (twap, reserve_price, max_return)", job.job_id);
                    } else {
                        warn!("Job ID: {} is missing some components, but will attempt to process with available data", job.job_id);
                    }

                    // Start the proof generation with timeout
                    let proof_result = tokio::time::timeout(
                        timeout_duration,
                        proof_provider.generate_proofs_from_data(timestamp_ranges),
                    )
                    .await;

                    // Always remove this job from the processing set when done
                    let _remove_result = {
                        let mut processing_jobs = processing_jobs.lock().await;
                        processing_jobs.remove(&job.job_id)
                    };

                    match proof_result {
                        Ok(Ok(receipt)) => {
                            info!("Proof generation successful for job ID: {}", job.job_id);
                            // Remove from failure tracking on success
                            {
                                let mut failures = job_failures.lock().await;
                                failures.remove(&job.job_id);
                            }
                            // Use the same job ID for the ProofGenerated to maintain consistency
                            let proof_generated = Job::ProofGenerated(Box::new(ProofGenerated {
                                job_id: job.job_id.clone(),
                                receipt,
                            }));

                            if let Err(e) = send_job_to_queue(&queue_clone, &proof_generated).await {
                                error!("Failed to send proof generated to queue: {}", e);
                            } else {
                                info!("Successfully sent proof result to queue for job ID: {}", job.job_id);
                                // Delete the message on success
                                if let Err(e) = queue_clone.delete_message(&message_clone).await {
                                    error!("Error deleting processed message from queue: {}", e);
                                } else {
                                    info!("Successfully deleted message from queue for job ID: {}", job.job_id);
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Error generating proofs for job ID {}: {}", job.job_id, e);
                            // Increment failure count
                            {
                                let mut failures = job_failures.lock().await;
                                let failure_entry = failures.entry(job.job_id.clone())
                                    .or_insert_with(|| JobProcessingState {
                                        failure_count: 0,
                                        last_failure_time: std::time::Instant::now(),
                                    });
                                failure_entry.failure_count += 1;
                                failure_entry.last_failure_time = std::time::Instant::now();
                                info!("Job ID {} has failed {} times", job.job_id, failure_entry.failure_count);
                            }
                            // Delete if we've reached max failures
                            if should_delete_after_processing {
                                info!("Forcibly deleting message for job ID {} after {} failures", job.job_id, max_failures);
                                if let Err(e) = queue_clone.delete_message(&message_clone).await {
                                    error!("Error deleting failed message from queue: {}", e);
                                } else {
                                    info!("Successfully deleted failed message from queue for job ID: {}", job.job_id);
                                    // Also remove from failure tracking
                                    let mut failures = job_failures.lock().await;
                                    failures.remove(&job.job_id);
                                }
                            } else {
                                // Message will be reprocessed after visibility timeout
                                info!("Message will be reprocessed after visibility timeout for job ID: {}", job.job_id);
                            }
                        }
                        Err(_) => {
                            error!("Proof generation timed out after {:?} for job ID: {}", timeout_duration, job.job_id);
                            // Increment failure count for timeouts too
                            {
                                let mut failures = job_failures.lock().await;
                                let failure_entry = failures.entry(job.job_id.clone())
                                    .or_insert_with(|| JobProcessingState {
                                        failure_count: 0,
                                        last_failure_time: std::time::Instant::now(),
                                    });
                                failure_entry.failure_count += 1;
                                failure_entry.last_failure_time = std::time::Instant::now();
                                info!("Job ID {} has timed out {} times", job.job_id, failure_entry.failure_count);
                            }
                            // Delete if we've reached max failures
                            if should_delete_after_processing {
                                info!("Forcibly deleting message for job ID {} after {} timeouts", job.job_id, max_failures);
                                if let Err(e) = queue_clone.delete_message(&message_clone).await {
                                    error!("Error deleting timed out message from queue: {}", e);
                                } else {
                                    info!("Successfully deleted timed out message from queue for job ID: {}", job.job_id);
                                    // Also remove from failure tracking
                                    let mut failures = job_failures.lock().await;
                                    failures.remove(&job.job_id);
                                }
                            } else {
                                // Message will be reprocessed after visibility timeout
                                info!("Message will be reprocessed after visibility timeout for job ID: {}", job.job_id);
                            }
                        }
                    };
                });
            }
        }

        // When the loop is aborted, wait for all tasks to finish
        join_set.join_all().await;

        Ok(())
    }
}

async fn send_job_to_queue<Q: Queue>(queue: &Arc<Q>, job: &Job) -> Result<()> {
    let job_str =
        serde_json::to_string(job).map_err(|e| eyre!("Failed to serialize job: {}", e))?;

    queue
        .send_message(job_str)
        .await
        .map_err(|e| eyre!("Failed to send message to queue: {}", e))
}

// Add this helper function to create ProofTimestampRanges from RequestProof
fn create_timestamp_ranges(job: &super::jobs::RequestProof) -> ProofTimestampRanges {
    // Use specific timestamp ranges if provided, otherwise fall back to the general ones
    let twap_start = job.twap_start_timestamp.unwrap_or(job.start_timestamp);
    let twap_end = job.twap_end_timestamp.unwrap_or(job.end_timestamp);

    let reserve_price_start = job
        .reserve_price_start_timestamp
        .unwrap_or(job.start_timestamp);
    let reserve_price_end = job.reserve_price_end_timestamp.unwrap_or(job.end_timestamp);

    let max_return_start = job
        .max_return_start_timestamp
        .unwrap_or(job.start_timestamp);
    let max_return_end = job.max_return_end_timestamp.unwrap_or(job.end_timestamp);

    ProofTimestampRanges::new(
        twap_start,
        twap_end,
        reserve_price_start,
        reserve_price_end,
        max_return_start,
        max_return_end,
    )
}

/**
 * Since we cannot really test this well, without a suitable source of bonsai mocking,
 * what we will do instead is to test the following:
 * 1. When a message is given, the handler will handle and process it.
 * 2. After the proof is complete, send the proof to the queue.
 * 3. Given some data, the prover will generate a (fake) proof.
 */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::message_queue::{QueueError, QueueMessage};
    use crate::{queue::local_message_queue::LocalMessageQueue, services::jobs::RequestProof};
    use risc0_zkvm::{Digest, FakeReceipt, InnerReceipt, MaybePruned, Receipt};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use tokio::time::sleep;

    // Test data available in the test db
    const START_TIMESTAMP: i64 = 1743249072;
    const END_TIMESTAMP: i64 = 1743249120;

    // Mock ProofProvider implementation for testing
    #[derive(Clone)]
    struct MockProofProvider {
        current_call_count: Arc<AtomicU32>,
        should_call_succeed_vec: Vec<bool>,
        delay: Duration,
    }

    impl MockProofProvider {
        fn new(should_call_succeed_vec: Vec<bool>, delay: Duration) -> Self {
            Self {
                current_call_count: Arc::new(AtomicU32::new(0)),
                should_call_succeed_vec,
                delay,
            }
        }
    }

    #[async_trait::async_trait]
    impl ProofProvider for MockProofProvider {
        async fn generate_proofs_from_data(
            &self,
            _timestamp_ranges: ProofTimestampRanges,
        ) -> Result<Receipt> {
            // Simulate some processing time
            sleep(self.delay).await;

            let current_count = self.current_call_count.fetch_add(1, Ordering::SeqCst);

            let should_succeed = match self.should_call_succeed_vec.get(current_count as usize) {
                Some(success) => *success,
                None => false,
            };

            if should_succeed {
                // Create a dummy receipt for testing
                let fake_receipt = FakeReceipt::new(MaybePruned::Pruned(Digest::ZERO));
                Ok(Receipt::new(InnerReceipt::Fake(fake_receipt), vec![]))
            } else {
                Err(eyre::eyre!("Mock proof generation failed"))
            }
        }

        fn is_disabled(&self) -> bool {
            false
        }
    }

    struct MockQueue;

    #[async_trait::async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, _message: String) -> Result<(), QueueError> {
            Err(QueueError::SendError("Mock error".to_string()))
        }

        async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
            Ok(vec![])
        }

        async fn delete_message(&self, _message: &QueueMessage) -> Result<(), QueueError> {
            Ok(())
        }
    }

    fn create_test_job(job_id: &str, start_timestamp: i64, end_timestamp: i64) -> RequestProof {
        RequestProof {
            job_id: job_id.to_string(),
            job_group_id: Some("test_group".to_string()),
            start_timestamp,
            end_timestamp,
            twap_start_timestamp: None,
            twap_end_timestamp: None,
            reserve_price_start_timestamp: None,
            reserve_price_end_timestamp: None,
            max_return_start_timestamp: None,
            max_return_end_timestamp: None,
        }
    }

    #[tokio::test]
    async fn test_successful_proof_generation() {
        // Create a test job
        let job = create_test_job("test_job_1", START_TIMESTAMP, END_TIMESTAMP);

        // Setup test components
        let queue = Arc::new(LocalMessageQueue::new());
        queue
            .send_message(serde_json::to_string(&Job::RequestProof(job.clone())).unwrap())
            .await
            .unwrap();

        let terminator = Arc::new(AtomicBool::new(false));
        let proof_provider: Arc<dyn ProofProvider + Send + Sync> = Arc::new(
            MockProofProvider::new(vec![true], Duration::from_millis(50)),
        );

        let handler = ProofJobHandler::new(
            queue.clone(),
            terminator.clone(),
            proof_provider,
            Duration::from_millis(50),
        );

        // Start the handler in a separate task
        let handle = tokio::spawn(async move { handler.receive_job().await });

        sleep(Duration::from_millis(200)).await;

        // Wait for the handler to finish
        terminator.store(true, Ordering::SeqCst);
        assert!(handle.await.is_ok());

        // Verify that a proof was sent back to the queue
        let messages = queue.receive_messages().await.unwrap();
        assert!(!messages.is_empty(), "No messages found in queue");

        let message = &messages[0];
        let received_job: Job = serde_json::from_str(&message.body).unwrap();

        match received_job {
            Job::ProofGenerated(proof) => {
                assert_eq!(proof.job_id, job.job_id, "Job ID mismatch");
            }
            _ => panic!("Expected ProofGenerated job, got {:?}", received_job),
        }
    }

    #[tokio::test]
    async fn test_failed_proof_generation_should_send_back_job() {
        // Create a test job
        let job = create_test_job("test_job_2", START_TIMESTAMP, END_TIMESTAMP);

        // Setup test components
        let queue = Arc::new(LocalMessageQueue::new());
        queue
            .send_message(serde_json::to_string(&Job::RequestProof(job.clone())).unwrap())
            .await
            .unwrap();

        let terminator = Arc::new(AtomicBool::new(false));
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![false],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(
            queue.clone(),
            terminator.clone(),
            proof_provider,
            Duration::from_millis(50),
        );

        // Start the handler in a separate task
        let handle = tokio::spawn(async move { handler.receive_job().await });

        // Give some time for processing
        sleep(Duration::from_millis(200)).await;

        // Terminate the handler
        terminator.store(true, Ordering::SeqCst);

        // Wait for the handler to finish
        assert!(handle.await.is_ok());

        // Verify that the job was requeued
        let messages = queue.receive_messages().await.unwrap();
        assert!(!messages.is_empty(), "No messages found in queue");

        let message = &messages[0];
        let received_job: Job = serde_json::from_str(&message.body).unwrap();

        match received_job {
            Job::RequestProof(requeued_job) => {
                assert_eq!(requeued_job.job_id, job.job_id, "Job ID mismatch");
                assert_eq!(
                    requeued_job.start_timestamp, job.start_timestamp,
                    "Start timestamp mismatch"
                );
                assert_eq!(
                    requeued_job.end_timestamp, job.end_timestamp,
                    "End timestamp mismatch"
                );
            }
            other_job => panic!("Expected RequestProof job, got {:?}", other_job),
        }
    }

    #[tokio::test]
    async fn test_invalid_message_handling_should_ignore_invalid_messages() {
        // Setup test components with invalid message
        let queue = Arc::new(LocalMessageQueue::new());
        queue
            .send_message("invalid json message".to_string())
            .await
            .unwrap();

        let terminator = Arc::new(AtomicBool::new(false));
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(
            queue.clone(),
            terminator.clone(),
            proof_provider,
            Duration::from_millis(50),
        );

        // Start the handler in a separate task
        let handle = tokio::spawn(async move { handler.receive_job().await });

        // Give some time for processing
        sleep(Duration::from_millis(100)).await;

        // Terminate the handler
        terminator.store(true, Ordering::SeqCst);

        // Wait for the handler to finish
        assert!(handle.await.is_ok());

        // Verify that no messages were processed
        let messages = queue.receive_messages().await.unwrap();
        assert!(messages.len() == 1, "Expected one junk messages in queue");
        assert!(messages[0].body == "invalid json message");
    }

    #[tokio::test]
    async fn test_successfully_handle_multiple_jobs() {
        // Create multiple test jobs
        let jobs = vec![
            create_test_job("test_job_1", START_TIMESTAMP, END_TIMESTAMP),
            create_test_job("test_job_2", START_TIMESTAMP, END_TIMESTAMP),
            create_test_job("test_job_3", START_TIMESTAMP, END_TIMESTAMP),
        ];

        // Setup test components
        let queue = Arc::new(LocalMessageQueue::new());
        for job in &jobs {
            queue
                .send_message(serde_json::to_string(&Job::RequestProof(job.clone())).unwrap())
                .await
                .unwrap();
        }

        let terminator = Arc::new(AtomicBool::new(false));
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true, true, true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(
            queue.clone(),
            terminator.clone(),
            proof_provider,
            Duration::from_millis(50),
        );

        // Start the handler in a separate task
        let handle = tokio::spawn(async move { handler.receive_job().await });

        // Give some time for processing
        sleep(Duration::from_millis(400)).await;

        // Terminate the handler
        terminator.store(true, Ordering::SeqCst);

        // Wait for the handler to finish
        assert!(handle.await.is_ok());

        // Verify that all jobs were processed and proofs were sent back
        let messages = queue.receive_messages().await.unwrap();
        assert_eq!(
            messages.len(),
            jobs.len(),
            "Expected {} messages, got {}",
            jobs.len(),
            messages.len()
        );

        // Parse all received messages into ProofGenerated jobs
        let mut received_proofs = Vec::new();
        for message in messages.iter() {
            let job: Job = serde_json::from_str(&message.body).unwrap();
            match job {
                Job::ProofGenerated(proof) => {
                    received_proofs.push(proof);
                }
                other_job => panic!("Expected ProofGenerated job, got {:?}", other_job),
            }
        }

        // Sort received proofs by job_id
        received_proofs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

        // Sort original jobs by job_id
        let mut sorted_jobs = jobs.clone();
        sorted_jobs.sort_by(|a, b| a.job_id.cmp(&b.job_id));

        // Now verify job IDs match in the sorted order
        for (proof, original_job) in received_proofs.iter().zip(sorted_jobs.iter()) {
            assert_eq!(proof.job_id, original_job.job_id, "Job ID mismatch");
        }
    }

    #[tokio::test]
    async fn test_mixed_success_failure_handling() {
        // Create test jobs that will succeed and fail
        let jobs = vec![
            create_test_job("success_job", START_TIMESTAMP, END_TIMESTAMP),
            create_test_job("failure_job", START_TIMESTAMP, END_TIMESTAMP),
            create_test_job("success_job_2", START_TIMESTAMP, END_TIMESTAMP),
        ];

        // Setup test components
        let queue = Arc::new(LocalMessageQueue::new());

        for job in &jobs {
            queue
                .send_message(serde_json::to_string(&Job::RequestProof(job.clone())).unwrap())
                .await
                .unwrap();
        }

        let terminator = Arc::new(AtomicBool::new(false));
        // Create a proof provider that fails for specific jobs
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true, false, true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(
            queue.clone(),
            terminator.clone(),
            proof_provider,
            Duration::from_millis(50),
        );

        // Start the handler in a separate task
        let handle = tokio::spawn(async move { handler.receive_job().await });

        // Give some time for processing
        sleep(Duration::from_millis(300)).await;

        // Terminate the handler
        terminator.store(true, Ordering::SeqCst);

        // Wait for the handler to finish
        assert!(handle.await.is_ok());

        // Verify that we have both successful proofs and requeued jobs
        let messages = queue.receive_messages().await.unwrap();
        assert!(!messages.is_empty(), "Expected messages in queue");

        let mut proof_count = 0;
        let mut requeue_count = 0;

        for message in messages {
            let job: Job = serde_json::from_str(&message.body).unwrap();
            match job {
                Job::ProofGenerated(_) => proof_count += 1,
                Job::RequestProof(_) => requeue_count += 1,
            }
        }

        assert!(proof_count > 0, "Expected at least one successful proof");
        assert!(requeue_count > 0, "Expected at least one requeued job");
    }

    // Tests for send_job_to_queue function
    #[tokio::test]
    async fn test_send_job_to_queue_success() {
        // Create a test job
        let job = Job::RequestProof(create_test_job(
            "test_job_1",
            START_TIMESTAMP,
            END_TIMESTAMP,
        ));
        let queue = Arc::new(LocalMessageQueue::new());

        // Send the job to queue
        assert!(send_job_to_queue(&queue, &job).await.is_ok());

        // Verify the message was sent correctly
        let messages = queue.receive_messages().await.unwrap();
        assert_eq!(messages.len(), 1, "Expected exactly one message in queue");

        // Verify the message content
        let received_job: Job = serde_json::from_str(&messages[0].body).unwrap();
        match received_job {
            Job::RequestProof(received) => {
                assert_eq!(received.job_id, "test_job_1");
                assert_eq!(received.start_timestamp, START_TIMESTAMP);
                assert_eq!(received.end_timestamp, END_TIMESTAMP);
            }
            _ => panic!("Expected RequestProof job, got {:?}", received_job),
        }
    }

    #[tokio::test]
    async fn test_send_job_to_queue_with_proof_generated() {
        // Create a test proof generated job
        let fake_receipt = FakeReceipt::new(MaybePruned::Pruned(Digest::ZERO));
        let receipt = Receipt::new(InnerReceipt::Fake(fake_receipt), vec![]);
        let job = Job::ProofGenerated(Box::new(ProofGenerated {
            job_id: "test_job_1".to_string(),
            receipt,
        }));

        let queue = Arc::new(LocalMessageQueue::new());

        // Send the job to queue
        assert!(send_job_to_queue(&queue, &job).await.is_ok());

        // Verify the message was sent correctly
        let messages = queue.receive_messages().await.unwrap();
        assert_eq!(messages.len(), 1, "Expected exactly one message in queue");

        // Verify the message content
        let received_job: Job = serde_json::from_str(&messages[0].body).unwrap();
        match received_job {
            Job::ProofGenerated(received) => {
                assert_eq!(received.job_id, "test_job_1");
            }
            _ => panic!("Expected ProofGenerated job, got {:?}", received_job),
        }
    }

    #[tokio::test]
    async fn test_send_job_to_queue_send_error() {
        // Create a test job
        let job = Job::RequestProof(create_test_job(
            "test_job_1",
            START_TIMESTAMP,
            END_TIMESTAMP,
        ));
        let queue = Arc::new(MockQueue {});

        // Attempt to send the job to queue
        let result = send_job_to_queue(&queue, &job).await;
        assert!(result.is_err(), "Expected error when sending fails");

        // Verify no messages were sent
        let messages = queue.receive_messages().await.unwrap();
        assert!(messages.is_empty(), "Expected no messages in queue");
    }
}
