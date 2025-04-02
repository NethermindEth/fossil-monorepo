use std::sync::{Arc, atomic::AtomicBool};

use crate::queue::message_queue::Queue;
use crate::{proof_composition::ProofProvider, services::jobs::ProofGenerated};
use db::DbConnection;
use db::models::get_block_base_fee_by_time_range;
use eyre::{Result, eyre};
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

use super::jobs::Job;

pub struct ProofJobHandler<
    Q: Queue + Send + Sync + 'static,
    P: ProofProvider + Send + Sync + 'static,
> {
    queue: Arc<Q>,
    terminator: Arc<AtomicBool>,
    db: Arc<DbConnection>,
    proof_provider: Arc<P>,
}

impl<Q, P> ProofJobHandler<Q, P>
where
    Q: Queue + Send + Sync + 'static,
    P: ProofProvider + Send + Sync + 'static,
{
    pub fn new(
        queue: Arc<Q>,
        terminator: Arc<AtomicBool>,
        db: Arc<DbConnection>,
        proof_provider: Arc<P>,
    ) -> Self {
        Self {
            queue,
            terminator,
            db,
            proof_provider,
        }
    }

    pub async fn receive_job(&self) -> Result<()> {
        // Create a join set to keep track of all the jobs;
        let mut join_set = JoinSet::new();
        while !self.terminator.load(std::sync::atomic::Ordering::Relaxed) {
            let messages = match self.queue.receive_messages().await {
                Ok(messages) => messages,
                Err(e) => {
                    warn!("Error receiving messages from queue: {}", e);
                    continue;
                }
            };

            for message in messages {
                let job: Job = match serde_json::from_str(&message.body) {
                    Ok(job) => job,
                    Err(e) => {
                        warn!("Error parsing job: {}", e);
                        // Ignoring any extra messages that are not valid
                        // TODO: maybe we should delete them?
                        continue;
                    }
                };

                // Only handle RequestProof jobs
                let job = match job {
                    Job::RequestProof(job) => job,
                    _ => continue,
                };

                // Take the job by deleting it from the queue
                if let Err(e) = self.queue.delete_message(&message).await {
                    error!("Error deleting message from queue: {}", e);
                    // We are continuing since other messages might still be valid
                    continue;
                };

                // Spawn a new task to handle the job
                let db_clone = self.db.clone();
                let queue_clone = self.queue.clone();
                let proof_provider = self.proof_provider.clone();

                join_set.spawn(async move {
                    debug!("Received & processing job: {:?}", job);

                    let block_base_fees = match get_block_base_fee_by_time_range(
                        db_clone,
                        job.start_timestamp,
                        job.end_timestamp,
                    )
                    .await
                    {
                        Ok(block_base_fees) => block_base_fees,
                        Err(e) => {
                            error!("Error getting block base fees: {}", e);

                            // Attempting to requeue the job
                            if let Err(e) =
                                send_job_to_queue(&queue_clone, &Job::RequestProof(job.clone()))
                                    .await
                            {
                                error!("Failed to requeue job: {}", e);
                            }
                            return;
                        }
                    };

                    if block_base_fees.is_empty() {
                        // Not retrying this in particular, as if the database is empty, it will likely
                        // remain empty.
                        warn!("No block base fees found for job: {:?}", job);
                        return;
                    }

                    // Start the proof generation
                    let proof_result = proof_provider
                        .generate_proofs_from_data(
                            job.start_timestamp,
                            job.end_timestamp,
                            block_base_fees,
                        )
                        .await;

                    match proof_result {
                        Ok(receipt) => {
                            // If successful, send the proof to the queue
                            let proof_generated = Job::ProofGenerated(Box::new(ProofGenerated {
                                job_id: job.clone().job_id,
                                receipt,
                            }));

                            if let Err(e) = send_job_to_queue(&queue_clone, &proof_generated).await
                            {
                                error!("Failed to send proof generated to queue: {}", e);

                                if let Err(e) =
                                    send_job_to_queue(&queue_clone, &Job::RequestProof(job.clone()))
                                        .await
                                {
                                    error!(
                                        "Failed to requeue job after proof generation success: {}",
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error generating proofs: {}", e);

                            if let Err(e) =
                                send_job_to_queue(&queue_clone, &Job::RequestProof(job)).await
                            {
                                error!("Failed to requeue job after proof generation error: {}", e);
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
            _start_timestamp: i64,
            _end_timestamp: i64,
            _raw_input: Vec<String>,
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
    }

    struct MockQueue;

    #[async_trait::async_trait]
    impl Queue for MockQueue {
        async fn send_message(&self, _message: String) -> Result<(), QueueError> {
            Err(QueueError::SendError(
                "Mock queue send message failed".to_string(),
            ))
        }

        async fn receive_messages(&self) -> Result<Vec<QueueMessage>, QueueError> {
            Ok(vec![])
        }

        async fn delete_message(&self, _message: &QueueMessage) -> Result<(), QueueError> {
            Err(QueueError::DeleteError(
                "Mock queue delete message failed".to_string(),
            ))
        }
    }

    // Helper function to setup test database
    async fn setup_db() -> Arc<DbConnection> {
        DbConnection::new("postgres://postgres:postgres@localhost:5432")
            .await
            .unwrap()
    }

    // Helper function to create a test job
    fn create_test_job(job_id: &str, start_timestamp: i64, end_timestamp: i64) -> RequestProof {
        RequestProof {
            job_group_id: None,
            job_id: job_id.to_string(),
            start_timestamp,
            end_timestamp,
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
        let db = setup_db().await;
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(queue.clone(), terminator.clone(), db, proof_provider);

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
        let db = setup_db().await;
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![false],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(queue.clone(), terminator.clone(), db, proof_provider);

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
        let db = setup_db().await;
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(queue.clone(), terminator.clone(), db, proof_provider);

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
        let db = setup_db().await;
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true, true, true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(queue.clone(), terminator.clone(), db, proof_provider);

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

        for (message, original_job) in messages.iter().zip(jobs.iter()) {
            let job: Job = serde_json::from_str(&message.body).unwrap();

            match job {
                Job::ProofGenerated(proof) => {
                    assert_eq!(proof.job_id, original_job.job_id, "Job ID mismatch");
                }
                other_job => panic!("Expected ProofGenerated job, got {:?}", other_job),
            }
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
        let db = setup_db().await;
        // Create a proof provider that fails for specific jobs
        let proof_provider = Arc::new(MockProofProvider::new(
            vec![true, false, true],
            Duration::from_millis(50),
        ));

        let handler = ProofJobHandler::new(queue.clone(), terminator.clone(), db, proof_provider);

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
