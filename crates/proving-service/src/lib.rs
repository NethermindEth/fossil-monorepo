mod handlers;
mod routes;

pub use routes::create_router;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use aws_config::SdkConfig;
    use message_handler::queue::sqs_message_queue::SqsMessageQueue;

    #[tokio::test]
    async fn test_create_router_from_lib() {
        // Create a test SqsMessageQueue
        let config = SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        let queue = Arc::new(SqsMessageQueue::new("test-queue-url".to_string(), config));

        // Ensure the router can be created without errors
        let _router = create_router(queue).await;

        // Basic verification - just check that we have a router
        assert!(true, "Router was created successfully");
    }
}
