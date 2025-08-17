#!/bin/bash

# Test script for message handler
# This script sends a test message to the SQS queue and then runs the message handler with proof generation disabled

# Get environment settings
source $(dirname "$0")/../.env

# Check if SQS queue URL is set
if [ -z "$SQS_QUEUE_URL" ]; then
    echo "Error: SQS_QUEUE_URL is not set in .env file"
    exit 1
fi

echo "Using SQS Queue: $SQS_QUEUE_URL"

# Ensure AWS CLI uses the right configuration for LocalStack
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1

# Function to purge the queue first (remove all messages)
purge_queue() {
    echo "Purging queue to remove old messages..."
    
    # Check if we're using localstack
    if [[ "$SQS_QUEUE_URL" == *"localhost"* ]]; then
        # Extract the endpoint URL
        ENDPOINT=$(echo $SQS_QUEUE_URL | grep -oP 'http://[^/]+')
        
        echo "Using LocalStack endpoint: $ENDPOINT"
        # Purge the queue
        RESULT=$(aws --endpoint-url="$ENDPOINT" sqs purge-queue \
            --queue-url "$SQS_QUEUE_URL" 2>&1)
        
        PURGE_EXIT_CODE=$?
        if [ $PURGE_EXIT_CODE -eq 0 ]; then
            echo "Queue purged successfully!"
        else
            echo "Failed to purge queue!"
            echo "Error: $RESULT"
        fi
    else
        # Use standard AWS CLI
        RESULT=$(aws sqs purge-queue \
            --queue-url "$SQS_QUEUE_URL" 2>&1)
        
        PURGE_EXIT_CODE=$?
        if [ $PURGE_EXIT_CODE -eq 0 ]; then
            echo "Queue purged successfully!"
        else
            echo "Failed to purge queue!"
            echo "Error: $RESULT"
        fi
    fi
    
    # Wait a moment for purge to complete
    echo "Waiting for purge to complete..."
    sleep 2
}

# Function to send a test message to the queue
send_test_message() {
    echo "Sending test message to queue..."
    
    # Generate a UUID for the job ID
    if command -v uuidgen >/dev/null 2>&1; then
        job_id=$(uuidgen)
    else
        # Fallback if uuidgen is not available
        timestamp=$(date +%s)
        job_id="test_job_$timestamp"
    fi
    
    echo "Using job ID: $job_id"
    
    # Create a FLAT JSON that exactly matches RequestProof struct
    # Include all timestamp ranges for a complete job 
    message_body=$(jq -n \
        --arg job_id "$job_id" \
        '{
            "job_id": $job_id,
            "job_group_id": $job_id,
            "start_timestamp": '"$(date +%s)"',
            "end_timestamp": '"$(($(date +%s) + 3600))"',
            "twap_start_timestamp": '"$(date +%s)"',
            "twap_end_timestamp": '"$(($(date +%s) + 3600))"',
            "reserve_price_start_timestamp": '"$(date +%s)"',
            "reserve_price_end_timestamp": '"$(($(date +%s) + 3600))"',
            "max_return_start_timestamp": '"$(date +%s)"',
            "max_return_end_timestamp": '"$(($(date +%s) + 3600))"'
        }'
    )

    echo "Sending message with body: $message_body"
    
    # For direct inspection - write the message to a file we can examine
    echo "$message_body" > /tmp/message_debug.json
    
    # It seems the AWS CLI might be wrapping our message in {"RequestProof":{...}}
    # Let's try directly constructing the message as a string, not a file

    # Check if we're using localstack
    if [[ "$SQS_QUEUE_URL" == *"localhost"* ]]; then
        # Extract the endpoint URL from SQS_QUEUE_URL
        # SQS_QUEUE_URL format is typically http://localhost:4567/000000000000/fossilQueue
        ENDPOINT=$(echo $SQS_QUEUE_URL | grep -oP 'http://[^/]+')
        
        echo "Using LocalStack endpoint: $ENDPOINT"
        # Use AWS CLI with the extracted endpoint and direct string message body
        # Double-quote the message_body to preserve the JSON structure
        RESULT=$(aws --endpoint-url="$ENDPOINT" sqs send-message \
            --queue-url "$SQS_QUEUE_URL" \
            --message-body "$message_body" 2>&1)
        
        SQS_EXIT_CODE=$?
        if [ $SQS_EXIT_CODE -eq 0 ]; then
            echo "Message sent successfully!"
            echo "Result: $RESULT"
        else
            echo "Failed to send message to SQS!"
            echo "Error: $RESULT"
            echo "Continuing anyway..."
        fi
    else
        # Use standard AWS CLI
        RESULT=$(aws sqs send-message \
            --queue-url "$SQS_QUEUE_URL" \
            --message-body "$message_body" 2>&1)
        
        SQS_EXIT_CODE=$?
        if [ $SQS_EXIT_CODE -eq 0 ]; then
            echo "Message sent successfully!"
            echo "Result: $RESULT"
        else
            echo "Failed to send message to SQS!"
            echo "Error: $RESULT"
            echo "Continuing anyway..."
        fi
    fi

    echo "Test message sent"
}

# Send the test message
if command -v jq >/dev/null 2>&1 && command -v aws >/dev/null 2>&1; then
    purge_queue
    send_test_message
else
    echo "Warning: jq or aws CLI not found, skipping test message"
    echo "To install: sudo apt-get install jq awscli"
fi

# Run the message handler with proof generation disabled
echo "Running message handler with simple mock proof provider..."
$(dirname "$0")/run-message-handler.sh --simple-mock 