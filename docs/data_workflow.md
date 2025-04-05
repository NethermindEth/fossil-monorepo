# Data Workflow: Offchain Processor to Proving Service

This document describes the end-to-end data workflow in the Fossil system, from the initial request to the offchain processor, through the proving service, and finally to the message handler.

## System Overview

The Fossil system consists of three main components:

1. **Offchain Processor**: Handles initial data requests and forwards them to the Proving Service
2. **Proving Service**: Manages the API for job submission and queues jobs for processing
3. **Message Handler**: Processes jobs from the queue, generates proofs, and handles the results

## Data Flow Diagram

```
┌─────────────────┐     HTTP     ┌─────────────────┐     SQS     ┌─────────────────┐
│                 │    Request    │                 │   Message   │                 │
│    Offchain     │──────────────▶│    Proving      │────────────▶│    Message      │
│    Processor    │              │    Service      │             │    Handler      │
│                 │◀──────────────│                 │◀────────────│                 │
└─────────────────┘    Response   └─────────────────┘   Results   └─────────────────┘
```

## Detailed Workflow

### 1. Initial Request to Offchain Processor

A client sends a pricing data request to the Offchain Processor's HTTP API endpoint:

```
POST http://localhost:3000/pricing_data
```

With a JSON body containing:
- Asset identifiers
- Parameters for different calculations (TWAP, volatility, reserve price)
- Client information

Example request:
```json
{
  "identifiers": ["0x50495443485f4c414b455f5631"],
  "params": {
    "twap": [1672531200, 1672574400],
    "volatility": [1672531200, 1672574400],
    "reserve_price": [1672531200, 1672574400]
  },
  "client_info": {
    "client_address": "0x018df581fe0ee497a4a3595cf62aea0bafa7ba1a54a7dcbafca37bfada67c718",
    "vault_address": "0x07b0110e7230a20881e57804d68e640777f4b55b487321556682e550f93fec7c",
    "timestamp": 1741243059
  }
}
```

### 2. Offchain Processor Handling

1. The Offchain Processor receives the request and validates it.
2. It generates a unique `job_id` to track the request.
3. The request is stored in the Offchain Processor's database with status `Pending`.
4. The Offchain Processor transforms the request into a format expected by the Proving Service.

### 3. Request to Proving Service

The Offchain Processor calls the Proving Service API:

```
POST http://127.0.0.1:3000/api/job
```

With a transformed payload:
```json
{
  "job_group_id": "<job_id>",
  "twap": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672574400
  },
  "reserve_price": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672574400
  },
  "max_return": {
    "start_timestamp": 1672531200,
    "end_timestamp": 1672574400
  }
}
```

### 4. Proving Service API Handling

1. The Proving Service receives the job request.
2. It creates three separate jobs (TWAP, reserve price, max return) with the same job group ID.
3. Each job is dispatched to an AWS SQS queue for processing.
4. The Proving Service responds with a success status and the job group ID:

```json
{
  "status": "success",
  "message": "All jobs dispatched successfully",
  "job_group_id": "<job_id>"
}
```

### 5. Message Handler Processing

1. The Message Handler service continuously polls the SQS queue for new jobs.
2. When it receives a job, it:
   - Parses the job from the message body
   - Removes the message from the queue to prevent reprocessing
   - Creates a task to handle the job asynchronously

3. For each job, the Message Handler:
   - Retrieves required data from the database based on the specified time range
   - Passes the data to the proof provider for proof generation
   - Handles proof generation with a timeout (default: 5 minutes)

4. After proof generation:
   - If successful, the proof is packaged as a `ProofGenerated` job and sent back to the queue
   - If unsuccessful or timed out, the original job is requeued for retry

### 6. Results Processing

1. The Proving Service monitors the queue for completed proofs.
2. When all proofs for a job group are completed, it updates the job status.
3. The Offchain Processor can poll the Proving Service for job status or receive callbacks.

## Error Handling

The system includes comprehensive error handling at multiple levels:

1. **Request validation**: Both services validate incoming requests and return appropriate error responses.
2. **Job tracking**: Failed jobs are marked with appropriate status codes and error messages.
3. **Retries**: Failed proof generation jobs can be requeued for retry.
4. **Timeouts**: Proof generation has configurable timeouts to prevent infinite processing.

## Technologies Used

- **HTTP Communication**: RESTful APIs for service-to-service communication
- **Message Queue**: AWS SQS for asynchronous job processing
- **Database**: PostgreSQL for persistent storage of job data and results
- **Proof Generation**: Uses the Bonsai Prover system with the RISC Zero zkVM 