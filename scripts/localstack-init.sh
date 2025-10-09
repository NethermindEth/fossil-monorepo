#!/bin/bash

echo "Initializing LocalStack services..."

# Wait for LocalStack to be ready
echo "Waiting for LocalStack to be ready..."
while ! awslocal sqs list-queues &>/dev/null; do
    echo "LocalStack not ready, waiting..."
    sleep 2
done

echo "LocalStack is ready, creating SQS queue..."

# Create the SQS queue
awslocal sqs create-queue --queue-name fossilQueue

echo "SQS queue 'fossilQueue' created successfully"

# List queues to verify
echo "Available queues:"
awslocal sqs list-queues
