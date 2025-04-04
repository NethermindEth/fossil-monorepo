#!/bin/bash

# Function to check if LocalStack is running
function is_localstack_running() {
  curl -s http://localhost:4567/_localstack/health | grep -q "\"services\"" && return 0 || return 1
}

# Function to check if SQS is enabled in LocalStack
function check_sqs_enabled() {
  curl -s http://localhost:4567/_localstack/health | grep -q "\"sqs\": \"running\|available\"" && return 0 || return 1
}

# Set a timeout (in seconds)
TIMEOUT=30
COUNT=0

# Wait for LocalStack to be ready
echo "Waiting for LocalStack to be ready..."
while [ $COUNT -lt $TIMEOUT ]; do
  echo "Attempt $COUNT of $TIMEOUT..."
  
  # Check if LocalStack is up by trying to access the health endpoint
  HEALTH_RESPONSE=$(curl -s http://localhost:4567/_localstack/health || echo "connection failed")
  echo "Health response: $HEALTH_RESPONSE"
  
  # In newer versions of LocalStack, SQS shows as "available" instead of "running"
  if echo "$HEALTH_RESPONSE" | grep -q "\"sqs\": \"running\"" || echo "$HEALTH_RESPONSE" | grep -q "\"sqs\": \"available\""; then
    echo "LocalStack SQS is available!"
    break
  fi
  
  # If we're getting a response but SQS isn't ready yet
  if echo "$HEALTH_RESPONSE" | grep -q "sqs"; then
    echo "LocalStack is up but SQS is not available yet. Waiting..."
  else
    echo "LocalStack is not responding properly. Check if the container is running:"
    docker ps | grep localstack || echo "No LocalStack container found."
    echo "Try starting it with: docker-compose -f docker/docker-compose.sqs.yml up -d"
  fi
  
  COUNT=$((COUNT + 1))
  sleep 1
done

if [ $COUNT -eq $TIMEOUT ]; then
  echo "Timed out waiting for LocalStack. Check if the container is running correctly."
  echo "Try starting it with: docker-compose -f docker/docker-compose.sqs.yml up -d"
  exit 1
fi

echo "LocalStack is ready!"

# Set up AWS CLI for LocalStack
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_DEFAULT_REGION=us-east-1
export AWS_ENDPOINT_URL=http://localhost:4567

# Create SQS queue
echo "Creating SQS queue 'fossilQueue'..."
aws --endpoint-url=http://localhost:4567 sqs create-queue --queue-name fossilQueue

# Verify queue creation
if [ $? -eq 0 ]; then
  echo "Queue created successfully!"
  echo "SQS URL: http://localhost:4567/000000000000/fossilQueue"
else
  echo "Failed to create queue. Check if AWS CLI is installed and LocalStack is functioning correctly."
  exit 1
fi

if ! is_localstack_running; then
    echo "LocalStack is not running!"
    echo "Try starting it with: docker-compose -f docker/docker-compose.sqs.yml up -d"
    exit 1
fi

# Check if SQS is enabled
if ! check_sqs_enabled; then
    echo "SQS is not enabled in LocalStack!"
    echo "Try starting it with: docker-compose -f docker/docker-compose.sqs.yml up -d"
    exit 1
fi 