# Deployment Guide

This comprehensive guide covers all deployment options for the Fossil monorepo, from local development to production environments on AWS ECS.

## Table of Contents

- [Overview](#overview)
- [Deployment Options](#deployment-options)
- [Local Deployment](#local-deployment)
- [Staging Deployment](#staging-deployment)
- [Production Deployment (AWS ECS)](#production-deployment-aws-ecs)
- [CI/CD Integration](#cicd-integration)
- [Monitoring and Observability](#monitoring-and-observability)
- [Rollback Procedures](#rollback-procedures)
- [Security Considerations](#security-considerations)
- [Troubleshooting Deployment Issues](#troubleshooting-deployment-issues)

## Overview

The Fossil monorepo consists of three main deployable components:

1. **Proving Service API** (`proving-service-api`) - HTTP API for job management on port 3001
2. **Message Handler** (`message-handler`) - Background worker for proof generation and verification
3. **Fossil API** (`fossil-api`) - HTTP API for pricing data requests on port 3000

### Architecture Overview

```
                          ┌──────────────────┐
                          │   Fossil API     │
                          │   (Port 3000)    │
                          └────────┬─────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │                             │
         ┌──────────▼─────────┐       ┌─────────▼──────────┐
         │ Fossil API         │       │  Proving Service   │
         │ Database (RDS)     │       │  API (Port 3001)   │
         └────────────────────┘       └──────────┬─────────┘
                                                  │
                                       ┌──────────┴─────────┐
                                       │                    │
                            ┌──────────▼──────────┐  ┌─────▼────────┐
                            │ Message Handler     │  │ Proving Svc  │
                            │ (Background Worker) │  │ Database     │
                            └──────────┬──────────┘  └──────────────┘
                                       │
                            ┌──────────┴──────────┐
                            │                     │
                    ┌───────▼────────┐    ┌──────▼──────┐
                    │  AWS SQS       │    │  StarkNet   │
                    │  Queue         │    │  Network    │
                    └────────────────┘    └─────────────┘
```

### Deployment Targets

| Environment | Purpose | Infrastructure | Network |
|-------------|---------|----------------|---------|
| Local | Development & Testing | Docker Compose + Katana | Devnet |
| Staging | Integration Testing | AWS ECS + Sepolia | Sepolia Testnet |
| Production | Live System | AWS ECS + Mainnet | StarkNet Mainnet |

## Deployment Options

### 1. Docker Compose (Local Development)

**Best for:**
- Local development
- Integration testing
- Quick prototyping

**Pros:**
- Fast setup with `make dev-up`
- Automatic contract deployment
- Full environment isolation
- Easy cleanup

**Cons:**
- Not suitable for production
- Limited scalability
- No high availability

**See:** [Local Deployment](#local-deployment)

### 2. Native Deployment

**Best for:**
- Development iteration
- Debugging services
- Testing individual components

**Pros:**
- Faster compile times
- Direct debugging access
- Lower resource usage
- Immediate code changes

**Cons:**
- Manual dependency management
- Port conflicts possible
- Environment setup complexity

**See:** [Environment Setup Guide](environment-setup.md#native-development-no-docker-for-services)

### 3. AWS ECS (Production)

**Best for:**
- Production deployments
- Staging environments
- High availability requirements

**Pros:**
- Auto-scaling capabilities
- Load balancing
- Service discovery
- Integrated monitoring
- Managed infrastructure

**Cons:**
- Higher cost
- More complex setup
- AWS-specific knowledge required

**See:** [Production Deployment (AWS ECS)](#production-deployment-aws-ecs)

### 4. Kubernetes (Future)

**Best for:**
- Multi-cloud deployments
- Advanced orchestration needs
- Large-scale deployments

**Status:** Not currently implemented. ECS provides equivalent functionality for AWS deployments.

## Local Deployment

Local deployment uses Docker Compose to run all services on your development machine with a local StarkNet devnet (Katana).

### Quick Start

```bash
# Initial setup (run once)
make setup

# Start all services
make dev-up

# Stop all services
make dev-down
```

### What `make dev-up` Does

1. **Starts Infrastructure Services**
   - Katana (StarkNet devnet) on port 5050
   - PostgreSQL databases:
     - Proving Service DB: port 5435
     - Fossil API DB: port 5434
     - Indexer DB: port 5433
   - LocalStack (AWS SQS emulation) on port 4567

2. **Deploys Smart Contracts**
   - Universal ECIP verifier
   - Groth16 verifier
   - PitchLake verifier
   - Hash storage contract
   - Fossil store contract
   - Multiple vault configurations (12min, 3hour, 1month)
   - Updates `.env.local` and `.env.docker` with deployed addresses

3. **Starts Application Services**
   - Fossil API (port 3000)
   - Proving Service API (port 3001)
   - Message Handler (background worker)

### Environment Configuration

Local deployment uses `.env.docker` for containerized services. Key settings:

```bash
# StarkNet
STARKNET_RPC_URL=http://katana:5050
NETWORK=DEVNET_KATANA

# Databases (Docker service names)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://postgres:postgres@fossil_api_db:5432/postgres
PROVING_SERVICE_DATABASE_URL=postgresql://postgres:postgres@proving_service_db:5432/postgres

# Services
PROVING_SERVICE_URL=http://proving-service-api:3001

# AWS (LocalStack)
AWS_ENDPOINT_URL=http://localstack:4566
SQS_QUEUE_URL=http://localstack:4566/000000000000/fossilQueue

# Features
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
```

**For detailed environment configuration, see:** [Environment Setup Guide](environment-setup.md#local-development-docker)

### Testing Local Deployment

```bash
# Automated comprehensive test
./test-local-request.sh

# Manual health checks
curl http://localhost:3000/health
curl http://localhost:3001/health

# Check service logs
docker logs fossil-monorepo-fossil-api-1 -f
docker logs fossil-monorepo-message-handler-1 -f
docker logs fossil-monorepo-proving-service-api-1 -f
```

### Docker Compose Files

| File | Purpose |
|------|---------|
| `docker-compose.local.yml` | Main development environment |
| `docker-compose.deploy.yml` | Contract deployment service |
| `docker-compose.services.yml` | Application services only |

### Common Local Deployment Commands

```bash
# Restart specific service
docker compose -f docker-compose.local.yml restart fossil-api

# View all container status
docker compose -f docker-compose.local.yml ps

# Clean up volumes (fresh start)
make dev-down  # Includes volume cleanup

# Rebuild services after code changes
docker compose -f docker-compose.local.yml up --build -d fossil-api

# Access database directly
psql postgresql://postgres:postgres@localhost:5434/postgres
```

## Staging Deployment

Staging deployment uses AWS ECS with Sepolia testnet for integration testing before production release.

### Sepolia Configuration

Staging uses real StarkNet testnet (Sepolia) and AWS services, but with test credentials and contracts.

### Environment Setup

Create `.env.sepolia` with staging configuration:

```bash
# StarkNet Sepolia
STARKNET_RPC_URL=https://starknet-sepolia.public.blastapi.io/rpc/v0_7
NETWORK=SEPOLIA

# Sepolia Account (from wallet)
STARKNET_ACCOUNT_ADDRESS=0x<your_sepolia_account>
STARKNET_PRIVATE_KEY=0x<your_sepolia_private_key>

# Deployed Sepolia Contracts
UNIVERSAL_ECIP_CONTRACT=0x<sepolia_address>
GROTH16_VERIFIER_CONTRACT=0x<sepolia_address>
PITCHLAKE_VERIFIER_CONTRACT=0x<sepolia_address>
HASH_STORAGE_ADDRESS=0x<sepolia_address>
FOSSIL_STORE_ADDRESS=0x<sepolia_address>
PITCHLAKE_VAULT_12MIN=0x<sepolia_address>

# Staging Databases (RDS)
OFFCHAIN_PROCESSOR_DATABASE_URL=postgresql://user:pass@staging-db.region.rds.amazonaws.com:5432/fossil_api
PROVING_SERVICE_DATABASE_URL=postgresql://user:pass@staging-db.region.rds.amazonaws.com:5432/proving_service

# AWS SQS (Staging Queue)
AWS_REGION=us-east-1
AWS_ENDPOINT_URL=  # Empty for real AWS
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/<account-id>/fossilQueue-sepolia
AWS_ACCESS_KEY_ID=<staging-iam-key>
AWS_SECRET_ACCESS_KEY=<staging-iam-secret>

# Service URLs
PROVING_SERVICE_URL=http://<staging-proving-service-internal>:3001

# Bonsai (Production API)
BONSAI_API_KEY="<your_bonsai_api_key>"
BONSAI_API_URL="https://api.bonsai.xyz/"

# CORS
ALLOWED_ORIGINS=https://dev.pitchlake.io,https://sepolia.pitchlake.io

# Features
ENABLE_PROOF=true
USE_RISC0_INTEGRATION=true
VERIFY_PROOFS_ONCHAIN=true
USE_MOCK_PRICING_DATA=false

# Logging
RUST_LOG=info
SERVER_PORT=3000  # or 3001 for proving service
```

### Staging Deployment Process

1. **Deploy Contracts to Sepolia**

```bash
# Using Starknet Foundry
cd starknet-contracts
snforge deploy --network sepolia

# Update .env.sepolia with deployed addresses
```

2. **Set Up Staging Database**

```bash
# Create RDS instance for staging
aws rds create-db-instance \
  --db-instance-identifier fossil-staging-db \
  --db-instance-class db.t3.micro \
  --engine postgres \
  --master-username postgres \
  --master-user-password <secure-password> \
  --allocated-storage 20

# Run migrations
cd fossil-api && sqlx migrate run --database-url $OFFCHAIN_PROCESSOR_DATABASE_URL
```

3. **Deploy to ECS**

Follow the [Production Deployment (AWS ECS)](#production-deployment-aws-ecs) instructions but use:
- Staging task definitions
- Sepolia environment variables
- Staging security groups and networking
- Lower resource allocations

4. **Verify Deployment**

```bash
# Test staging endpoints
curl https://staging.fossil-api.example.com/health
curl https://staging.proving-service.example.com/health

# Submit test job
./test-staging-request.sh
```

### Staging Best Practices

1. **Use Separate AWS Resources**
   - Dedicated RDS instances
   - Separate SQS queues
   - Isolated VPCs/subnets

2. **Test Real-World Scenarios**
   - Actual proof generation
   - StarkNet transaction submission
   - Full end-to-end flows

3. **Monitor Sepolia Resources**
   - ETH balance for transactions
   - Contract gas costs
   - API rate limits

4. **Automated Testing**
   - Run integration tests against staging
   - Validate contract interactions
   - Test rollback procedures

## Production Deployment (AWS ECS)

Production deployment uses AWS Elastic Container Service (ECS) with Fargate for a fully managed, scalable containerized deployment.

### Prerequisites

1. **AWS Account** with appropriate permissions
2. **Docker images** published to container registry (ECR or Docker Hub)
3. **RDS databases** provisioned and accessible
4. **SQS queues** created
5. **VPC** with public/private subnets configured
6. **StarkNet mainnet** contracts deployed
7. **Secrets** stored in AWS Secrets Manager

### AWS Infrastructure Setup

#### 1. VPC and Networking

```bash
# Create VPC
aws ec2 create-vpc --cidr-block 10.0.0.0/16 --tag-specifications 'ResourceType=vpc,Tags=[{Key=Name,Value=fossil-vpc}]'

# Create subnets
aws ec2 create-subnet --vpc-id <vpc-id> --cidr-block 10.0.1.0/24 --availability-zone us-east-1a
aws ec2 create-subnet --vpc-id <vpc-id> --cidr-block 10.0.2.0/24 --availability-zone us-east-1b

# Create internet gateway
aws ec2 create-internet-gateway --tag-specifications 'ResourceType=internet-gateway,Tags=[{Key=Name,Value=fossil-igw}]'
aws ec2 attach-internet-gateway --vpc-id <vpc-id> --internet-gateway-id <igw-id>

# Create security groups
aws ec2 create-security-group --group-name fossil-api-sg --description "Fossil API security group" --vpc-id <vpc-id>
aws ec2 authorize-security-group-ingress --group-id <sg-id> --protocol tcp --port 3000 --cidr 0.0.0.0/0
```

#### 2. RDS Database Setup

```bash
# Create DB subnet group
aws rds create-db-subnet-group \
  --db-subnet-group-name fossil-db-subnet \
  --db-subnet-group-description "Fossil DB subnet group" \
  --subnet-ids <subnet-1> <subnet-2>

# Create RDS instances (separate for each service)
aws rds create-db-instance \
  --db-instance-identifier fossil-api-db \
  --db-instance-class db.t3.medium \
  --engine postgres \
  --engine-version 15.4 \
  --master-username postgres \
  --master-user-password <secure-password> \
  --allocated-storage 100 \
  --storage-encrypted \
  --db-subnet-group-name fossil-db-subnet \
  --vpc-security-group-ids <db-sg-id>

# Repeat for proving service database
aws rds create-db-instance \
  --db-instance-identifier fossil-proving-db \
  # ... similar configuration
```

#### 3. SQS Queue Setup

```bash
# Create production queue
aws sqs create-queue --queue-name fossilQueue-prod

# Set queue attributes
aws sqs set-queue-attributes \
  --queue-url <queue-url> \
  --attributes VisibilityTimeout=300,MessageRetentionPeriod=1209600
```

#### 4. ECR Repository Setup

```bash
# Create repositories
aws ecr create-repository --repository-name fossil-proving-service-api
aws ecr create-repository --repository-name fossil-message-handler
aws ecr create-repository --repository-name fossil-api

# Get login credentials
aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com
```

#### 5. Build and Push Docker Images

```bash
# Build images
cd proving-service
docker build -t fossil-proving-service-api -f docker/Dockerfile.proving-service .
docker build -t fossil-message-handler -f docker/Dockerfile.message-handler .

cd ../fossil-api
docker build -t fossil-api -f docker/Dockerfile .

# Tag images
docker tag fossil-proving-service-api:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-proving-service-api:latest
docker tag fossil-message-handler:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-message-handler:latest
docker tag fossil-api:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-api:latest

# Push images
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-proving-service-api:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-message-handler:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-api:latest
```

### ECS Configuration

#### 1. Create ECS Cluster

```bash
aws ecs create-cluster --cluster-name fossil-cluster
```

#### 2. Create Task Definitions

**Proving Service API Task Definition:**

```json
{
  "family": "proving-service-api",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "executionRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskRole",
  "containerDefinitions": [
    {
      "name": "proving-service-api",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-proving-service-api:latest",
      "portMappings": [
        {
          "containerPort": 3001,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "RUST_LOG", "value": "info"},
        {"name": "SERVER_PORT", "value": "3001"},
        {"name": "AWS_REGION", "value": "us-east-1"}
      ],
      "secrets": [
        {"name": "PROVING_SERVICE_DATABASE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/proving-db-url"},
        {"name": "AWS_ACCESS_KEY_ID", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-key-id"},
        {"name": "AWS_SECRET_ACCESS_KEY", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-secret-key"},
        {"name": "SQS_QUEUE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/sqs-queue-url"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/proving-service-api",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

**Message Handler Task Definition:**

```json
{
  "family": "message-handler",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "2048",
  "memory": "4096",
  "executionRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskRole",
  "containerDefinitions": [
    {
      "name": "message-handler",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-message-handler:latest",
      "environment": [
        {"name": "ENABLE_PROOF", "value": "true"},
        {"name": "RUST_LOG", "value": "info"},
        {"name": "RUST_BACKTRACE", "value": "1"},
        {"name": "BONSAI_API_URL", "value": "https://api.bonsai.xyz/"},
        {"name": "STARKNET_RPC_URL", "value": "https://starknet-mainnet.public.blastapi.io/rpc/v0_7"},
        {"name": "NETWORK", "value": "MAINNET"},
        {"name": "VERIFY_PROOFS_ONCHAIN", "value": "true"},
        {"name": "STARKNET_MAX_RETRIES", "value": "5"},
        {"name": "STARKNET_INITIAL_BACKOFF_MS", "value": "1000"},
        {"name": "STARKNET_MAX_BACKOFF_MS", "value": "30000"}
      ],
      "secrets": [
        {"name": "PROVING_SERVICE_DATABASE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/proving-db-url"},
        {"name": "AWS_ACCESS_KEY_ID", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-key-id"},
        {"name": "AWS_SECRET_ACCESS_KEY", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-secret-key"},
        {"name": "SQS_QUEUE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/sqs-queue-url"},
        {"name": "BONSAI_API_KEY", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/bonsai-api-key"},
        {"name": "STARKNET_ACCOUNT_ADDRESS", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/starknet-address"},
        {"name": "STARKNET_PRIVATE_KEY", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/starknet-key"},
        {"name": "UNIVERSAL_ECIP_CLASS", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/ecip-contract"},
        {"name": "GROTH16_VERIFIER_CONTRACT", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/groth16-contract"},
        {"name": "PITCHLAKE_VERIFIER_CONTRACT", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/pitchlake-verifier"},
        {"name": "HASH_STORAGE_ADDRESS", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/hash-storage"},
        {"name": "FOSSIL_STORE_ADDRESS", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/fossil-store"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/message-handler",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

**Fossil API Task Definition:**

```json
{
  "family": "fossil-api",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "executionRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::<account-id>:role/ecsTaskRole",
  "containerDefinitions": [
    {
      "name": "fossil-api",
      "image": "<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-api:latest",
      "portMappings": [
        {
          "containerPort": 3000,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "RUST_LOG", "value": "info"},
        {"name": "SERVER_PORT", "value": "3000"},
        {"name": "STARKNET_RPC_URL", "value": "https://starknet-mainnet.public.blastapi.io/rpc/v0_7"},
        {"name": "ALLOWED_ORIGINS", "value": "https://pitchlake.io"},
        {"name": "AWS_REGION", "value": "us-east-1"}
      ],
      "secrets": [
        {"name": "OFFCHAIN_PROCESSOR_DATABASE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/api-db-url"},
        {"name": "PROVING_SERVICE_URL", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/proving-service-url"},
        {"name": "AWS_ACCESS_KEY_ID", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-key-id"},
        {"name": "AWS_SECRET_ACCESS_KEY", "valueFrom": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/aws-secret-key"}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/fossil-api",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

#### 3. Register Task Definitions

```bash
aws ecs register-task-definition --cli-input-json file://proving-service-api-task.json
aws ecs register-task-definition --cli-input-json file://message-handler-task.json
aws ecs register-task-definition --cli-input-json file://fossil-api-task.json
```

#### 4. Create ECS Services

```bash
# Proving Service API (with load balancer)
aws ecs create-service \
  --cluster fossil-cluster \
  --service-name proving-service-api \
  --task-definition proving-service-api \
  --desired-count 2 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[<subnet-1>,<subnet-2>],securityGroups=[<sg-id>],assignPublicIp=ENABLED}" \
  --load-balancers "targetGroupArn=<tg-arn>,containerName=proving-service-api,containerPort=3001"

# Message Handler (background worker, no load balancer)
aws ecs create-service \
  --cluster fossil-cluster \
  --service-name message-handler \
  --task-definition message-handler \
  --desired-count 1 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[<subnet-1>,<subnet-2>],securityGroups=[<sg-id>],assignPublicIp=ENABLED}"

# Fossil API (with load balancer)
aws ecs create-service \
  --cluster fossil-cluster \
  --service-name fossil-api \
  --task-definition fossil-api \
  --desired-count 2 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[<subnet-1>,<subnet-2>],securityGroups=[<sg-id>],assignPublicIp=ENABLED}" \
  --load-balancers "targetGroupArn=<tg-arn>,containerName=fossil-api,containerPort=3000"
```

### ECS Environment Variables Reference

This section contains the complete environment variable configuration for each ECS service. Values marked with `<placeholder>` should be replaced with your actual values.

#### Image 1: proving-service-api

**Image:** `<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-proving-service-api:latest` or `ametelnethermind/fossil-proving-service-api:latest`

**Port:** 3001

**Environment Variables:**
```bash
PROVING_SERVICE_DATABASE_URL=postgres://postgres:<password>@<rds-endpoint>:5432/postgres
DATABASE_URL=postgres://postgres:<password>@<rds-endpoint>:5432/postgres
AWS_ENDPOINT_URL=
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/<account-id>/fossilQueue-sepolia
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=<iam-key>
AWS_SECRET_ACCESS_KEY=<iam-secret>
RUST_LOG=info
SERVER_PORT=3001
```

**Notes:**
- `AWS_ENDPOINT_URL` should be empty for production (removes LocalStack)
- Use separate RDS instance for proving service database
- Configure IAM role instead of hardcoded AWS credentials when possible

#### Image 2: message-handler

**Image:** `<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-message-handler:latest` or `ametelnethermind/fossil-message-handler:latest`

**Port:** None (background worker)

**Environment Variables:**
```bash
PROVING_SERVICE_DATABASE_URL=postgres://postgres:<password>@<rds-endpoint>:5432/postgres
DATABASE_URL=postgres://postgres:<password>@<rds-endpoint>:5432/postgres
AWS_ENDPOINT_URL=
SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/<account-id>/fossilQueue-sepolia
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=<iam-key>
AWS_SECRET_ACCESS_KEY=<iam-secret>
ENABLE_PROOF=true
RUST_LOG=info
RUST_BACKTRACE=1
BONSAI_API_KEY=<your-bonsai-api-key>
BONSAI_API_URL=https://api.bonsai.xyz/
STARKNET_RPC_URL=https://starknet-sepolia.public.blastapi.io
STARKNET_ACCOUNT_ADDRESS=0x<your-starknet-account-address>
STARKNET_PRIVATE_KEY=0x<your-starknet-private-key>
UNIVERSAL_ECIP_CLASS=0x<deployed-contract-address>
GROTH16_VERIFIER_CONTRACT=0x<deployed-contract-address>
PITCHLAKE_VERIFIER_CONTRACT=0x<deployed-contract-address>
HASH_STORAGE_ADDRESS=0x<deployed-contract-address>
FOSSIL_STORE_ADDRESS=0x<deployed-contract-address>
NETWORK=SEPOLIA
ETH_RPC_URL=https://sepolia.infura.io/v3/<your-infura-key>
VERIFY_PROOFS_ONCHAIN=true
STARKNET_MAX_RETRIES=5
STARKNET_INITIAL_BACKOFF_MS=1000
STARKNET_MAX_BACKOFF_MS=30000
PITCHLAKE_VAULT=<deployed-vault-address>
PITCH_LAKE_VERIFIER_CONTRACT_ADDRESS=0x<deployed-contract-address>
```

**Important Notes:**
- This is the most complex service with the most environment variables
- All contract addresses must be deployed on the target network (Sepolia/Mainnet)
- Private keys should ALWAYS be stored in AWS Secrets Manager, not in task definition
- Consider using AWS IAM roles for AWS credentials instead of access keys

#### Image 3: fossil-api

**Image:** `<account-id>.dkr.ecr.us-east-1.amazonaws.com/fossil-api:latest` or `ametelnethermind/fossil-api:latest`

**Port:** 3000

**Environment Variables:**
```bash
OFFCHAIN_PROCESSOR_DATABASE_URL=postgres://postgres:<password>@<fossil-api-rds>:5432/postgres
DATABASE_URL=postgres://postgres:<password>@<fossil-api-rds>:5432/postgres
AWS_ENDPOINT_URL=
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=<iam-key>
AWS_SECRET_ACCESS_KEY=<iam-secret>
PROVING_SERVICE_URL=http://<proving-service-internal-url>:3001
RUST_LOG=info
SERVER_PORT=3000
STARKNET_RPC_URL=https://starknet-sepolia.public.blastapi.io
ALLOWED_ORIGINS=https://dev.pitchlake.io,https://sepolia.pitchlake.io
```

**Notes:**
- Use separate RDS instance from proving service database
- `PROVING_SERVICE_URL` should use internal ECS service discovery or load balancer
- `ALLOWED_ORIGINS` should be updated for production domains
- Only variables actually used by the code are listed above

### Production Best Practices

1. **High Availability**
   - Run at least 2 tasks per service across multiple AZs
   - Use Application Load Balancer for HTTP services
   - Configure health checks for all services

2. **Auto Scaling**

```bash
# Register scalable target
aws application-autoscaling register-scalable-target \
  --service-namespace ecs \
  --scalable-dimension ecs:service:DesiredCount \
  --resource-id service/fossil-cluster/fossil-api \
  --min-capacity 2 \
  --max-capacity 10

# Create scaling policy
aws application-autoscaling put-scaling-policy \
  --service-namespace ecs \
  --scalable-dimension ecs:service:DesiredCount \
  --resource-id service/fossil-cluster/fossil-api \
  --policy-name cpu-scaling \
  --policy-type TargetTrackingScaling \
  --target-tracking-scaling-policy-configuration file://scaling-policy.json
```

3. **Resource Allocation**
   - Proving Service API: 1-2 vCPU, 2-4GB RAM
   - Message Handler: 2-4 vCPU, 4-8GB RAM (proof generation is CPU intensive)
   - Fossil API: 1-2 vCPU, 2-4GB RAM

4. **Database Optimization**
   - Use RDS with Multi-AZ deployment
   - Enable automated backups (7-30 day retention)
   - Configure read replicas for high traffic
   - Use connection pooling

5. **Network Security**
   - Place services in private subnets
   - Use NAT gateway for outbound traffic
   - Restrict security group rules to minimum required
   - Enable VPC Flow Logs

6. **Secrets Management**
   - Store all sensitive values in AWS Secrets Manager
   - Use IAM roles instead of access keys where possible
   - Rotate secrets regularly
   - Enable secret rotation for RDS credentials

## CI/CD Integration

Automated deployment using GitHub Actions for continuous integration and deployment.

### GitHub Actions Workflow

The repository includes workflows for building, testing, and deploying services. Here's an example complete CI/CD pipeline:

**`.github/workflows/deploy-production.yml`:**

```yaml
name: Deploy to Production

on:
  push:
    branches:
      - main
  workflow_dispatch:

env:
  AWS_REGION: us-east-1
  ECR_REGISTRY: ${{ secrets.AWS_ACCOUNT_ID }}.dkr.ecr.us-east-1.amazonaws.com

jobs:
  test:
    name: Run Tests
    uses: ./.github/workflows/shared-test.yml

  build-and-push:
    name: Build and Push Images
    needs: test
    runs-on: ubuntu-24.04
    strategy:
      matrix:
        service:
          - name: fossil-api
            dockerfile: docker/Dockerfile
            context: fossil-api
          - name: proving-service-api
            dockerfile: docker/Dockerfile.proving-service
            context: proving-service
          - name: message-handler
            dockerfile: docker/Dockerfile.message-handler
            context: proving-service

    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: ${{ env.AWS_REGION }}

      - name: Login to Amazon ECR
        id: login-ecr
        uses: aws-actions/amazon-ecr-login@v2

      - name: Build, tag, and push image
        working-directory: ${{ matrix.service.context }}
        run: |
          IMAGE_TAG=${{ github.sha }}
          docker build -t ${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:$IMAGE_TAG -f ${{ matrix.service.dockerfile }} .
          docker tag ${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:$IMAGE_TAG ${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:latest
          docker push ${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:$IMAGE_TAG
          docker push ${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:latest
          echo "image=${{ env.ECR_REGISTRY }}/${{ matrix.service.name }}:$IMAGE_TAG" >> $GITHUB_OUTPUT

  deploy:
    name: Deploy to ECS
    needs: build-and-push
    runs-on: ubuntu-24.04
    strategy:
      matrix:
        service: [fossil-api, proving-service-api, message-handler]

    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: ${{ env.AWS_REGION }}

      - name: Download task definition
        run: |
          aws ecs describe-task-definition \
            --task-definition ${{ matrix.service }} \
            --query taskDefinition > task-definition.json

      - name: Update task definition with new image
        id: task-def
        uses: aws-actions/amazon-ecs-render-task-definition@v1
        with:
          task-definition: task-definition.json
          container-name: ${{ matrix.service }}
          image: ${{ env.ECR_REGISTRY }}/${{ matrix.service }}:${{ github.sha }}

      - name: Deploy to ECS
        uses: aws-actions/amazon-ecs-deploy-task-definition@v1
        with:
          task-definition: ${{ steps.task-def.outputs.task-definition }}
          service: ${{ matrix.service }}
          cluster: fossil-cluster
          wait-for-service-stability: true

      - name: Verify deployment
        run: |
          sleep 30
          aws ecs describe-services \
            --cluster fossil-cluster \
            --services ${{ matrix.service }} \
            --query 'services[0].deployments' \
            --output table
```

### Staging Deployment Workflow

**`.github/workflows/deploy-staging.yml`:**

```yaml
name: Deploy to Staging

on:
  push:
    branches:
      - develop
  workflow_dispatch:

env:
  AWS_REGION: us-east-1
  ECR_REGISTRY: ${{ secrets.AWS_ACCOUNT_ID }}.dkr.ecr.us-east-1.amazonaws.com
  ECS_CLUSTER: fossil-staging-cluster

jobs:
  deploy-staging:
    name: Deploy to Staging
    runs-on: ubuntu-24.04
    # Similar to production but with staging cluster/services
```

### Deployment Automation Scripts

**`scripts/deploy.sh`:**

```bash
#!/bin/bash
set -e

ENVIRONMENT=${1:-production}
IMAGE_TAG=${2:-latest}

echo "Deploying to $ENVIRONMENT with tag $IMAGE_TAG"

# Update task definitions
for service in fossil-api proving-service-api message-handler; do
  echo "Updating $service..."

  # Get current task definition
  aws ecs describe-task-definition \
    --task-definition $service \
    --query taskDefinition > /tmp/task-def.json

  # Update image tag in task definition
  jq --arg IMAGE_TAG "$IMAGE_TAG" \
    '.containerDefinitions[0].image |= sub(":[^:]+$"; ":" + $IMAGE_TAG)' \
    /tmp/task-def.json > /tmp/new-task-def.json

  # Register new task definition
  aws ecs register-task-definition \
    --cli-input-json file:///tmp/new-task-def.json

  # Update service
  aws ecs update-service \
    --cluster fossil-cluster \
    --service $service \
    --task-definition $service \
    --force-new-deployment
done

echo "Deployment initiated. Monitor progress:"
echo "aws ecs describe-services --cluster fossil-cluster --services fossil-api proving-service-api message-handler"
```

### Pre-Deployment Checklist

- [ ] All tests passing (`make test-all`)
- [ ] Linting passed (`make lint-all`)
- [ ] Database migrations prepared
- [ ] Secrets updated in AWS Secrets Manager
- [ ] Contract addresses verified for target network
- [ ] Environment variables reviewed
- [ ] Rollback plan prepared
- [ ] Team notified of deployment

### Post-Deployment Verification

```bash
# Check service health
aws ecs describe-services \
  --cluster fossil-cluster \
  --services fossil-api proving-service-api message-handler

# Check task status
aws ecs list-tasks --cluster fossil-cluster --service-name fossil-api
aws ecs describe-tasks --cluster fossil-cluster --tasks <task-arn>

# Test endpoints
curl https://api.fossil.example.com/health
curl https://proving-service.fossil.example.com/health

# Check logs
aws logs tail /ecs/fossil-api --follow
aws logs tail /ecs/message-handler --follow
```

## Monitoring and Observability

### CloudWatch Logs

All ECS services send logs to CloudWatch Logs.

**Log Groups:**
- `/ecs/fossil-api`
- `/ecs/proving-service-api`
- `/ecs/message-handler`

**Viewing Logs:**

```bash
# Tail logs in real-time
aws logs tail /ecs/fossil-api --follow

# Filter logs
aws logs filter-log-events \
  --log-group-name /ecs/message-handler \
  --filter-pattern "ERROR"

# Query logs with Insights
aws logs start-query \
  --log-group-name /ecs/message-handler \
  --start-time $(date -d '1 hour ago' +%s) \
  --end-time $(date +%s) \
  --query-string 'fields @timestamp, @message | filter @message like /proof generation/'
```

### CloudWatch Metrics

**Key Metrics to Monitor:**

1. **ECS Metrics**
   - CPUUtilization
   - MemoryUtilization
   - TaskCount
   - RunningTaskCount

2. **Application Load Balancer**
   - RequestCount
   - TargetResponseTime
   - HTTPCode_Target_2XX_Count
   - HTTPCode_Target_5XX_Count

3. **RDS Metrics**
   - DatabaseConnections
   - CPUUtilization
   - FreeStorageSpace
   - ReadLatency / WriteLatency

4. **SQS Metrics**
   - NumberOfMessagesSent
   - NumberOfMessagesReceived
   - ApproximateNumberOfMessagesVisible
   - ApproximateAgeOfOldestMessage

**Creating CloudWatch Dashboard:**

```bash
# Create dashboard JSON
cat > dashboard.json <<EOF
{
  "widgets": [
    {
      "type": "metric",
      "properties": {
        "metrics": [
          ["AWS/ECS", "CPUUtilization", {"stat": "Average"}],
          [".", "MemoryUtilization", {"stat": "Average"}]
        ],
        "period": 300,
        "stat": "Average",
        "region": "us-east-1",
        "title": "ECS Resource Utilization"
      }
    }
  ]
}
EOF

# Create dashboard
aws cloudwatch put-dashboard \
  --dashboard-name FossilProduction \
  --dashboard-body file://dashboard.json
```

### CloudWatch Alarms

**CPU Utilization Alert:**

```bash
aws cloudwatch put-metric-alarm \
  --alarm-name fossil-api-high-cpu \
  --alarm-description "Alert when CPU exceeds 80%" \
  --metric-name CPUUtilization \
  --namespace AWS/ECS \
  --statistic Average \
  --period 300 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 2 \
  --dimensions Name=ServiceName,Value=fossil-api Name=ClusterName,Value=fossil-cluster \
  --alarm-actions arn:aws:sns:us-east-1:<account-id>:fossil-alerts
```

**SQS Queue Depth Alert:**

```bash
aws cloudwatch put-metric-alarm \
  --alarm-name fossil-sqs-high-depth \
  --alarm-description "Alert when queue depth exceeds 100" \
  --metric-name ApproximateNumberOfMessagesVisible \
  --namespace AWS/SQS \
  --statistic Average \
  --period 300 \
  --threshold 100 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 1 \
  --dimensions Name=QueueName,Value=fossilQueue-prod \
  --alarm-actions arn:aws:sns:us-east-1:<account-id>:fossil-alerts
```

**Error Rate Alert:**

```bash
aws cloudwatch put-metric-alarm \
  --alarm-name fossil-api-high-errors \
  --alarm-description "Alert when 5xx errors exceed 10" \
  --metric-name HTTPCode_Target_5XX_Count \
  --namespace AWS/ApplicationELB \
  --statistic Sum \
  --period 300 \
  --threshold 10 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 1 \
  --alarm-actions arn:aws:sns:us-east-1:<account-id>:fossil-alerts
```

### Application-Level Monitoring

**Structured Logging:**

Ensure all services use structured logging with the `RUST_LOG` environment variable:

```bash
RUST_LOG=info,fossil_api=debug,message_handler=debug
```

**Custom Metrics:**

Services should emit custom metrics for:
- Job submission rate
- Proof generation success/failure rate
- Proof verification latency
- StarkNet transaction success rate

**Distributed Tracing:**

Consider implementing OpenTelemetry for distributed tracing across services.

### Third-Party Monitoring (Optional)

**DataDog Integration:**

```bash
# Add DataDog agent as sidecar container
{
  "name": "datadog-agent",
  "image": "datadog/agent:latest",
  "environment": [
    {"name": "DD_API_KEY", "value": "<datadog-api-key>"},
    {"name": "ECS_FARGATE", "value": "true"}
  ]
}
```

**Prometheus Integration:**

Export metrics from services and scrape with Prometheus:

```rust
// In Rust service
use prometheus::{Encoder, TextEncoder, Counter, Registry};

let registry = Registry::new();
let counter = Counter::new("http_requests_total", "Total HTTP requests")?;
registry.register(Box::new(counter.clone()))?;
```

## Rollback Procedures

### Rolling Back an ECS Deployment

#### Option 1: Revert to Previous Task Definition

```bash
# List recent task definition revisions
aws ecs list-task-definitions \
  --family-prefix fossil-api \
  --sort DESC \
  --max-items 5

# Update service to use previous revision
aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --task-definition fossil-api:42  # Previous working revision

# Force new deployment
aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --force-new-deployment
```

#### Option 2: Deploy Previous Docker Image

```bash
# Use specific image tag (previous deployment)
./scripts/deploy.sh production v1.2.3

# Or manually update task definition
aws ecs register-task-definition \
  --cli-input-json file://task-definition-v1.2.3.json

aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --task-definition fossil-api
```

#### Option 3: GitHub Actions Rollback

Trigger deployment workflow with previous commit SHA:

```bash
# Via GitHub CLI
gh workflow run deploy-production.yml -f commit_sha=abc123def

# Or revert commit and push
git revert <bad-commit-sha>
git push origin main  # Triggers automatic deployment
```

### Database Rollback

**If database migrations were applied:**

```bash
# Connect to database
psql $OFFCHAIN_PROCESSOR_DATABASE_URL

# Revert migration (if using sqlx)
cd fossil-api
sqlx migrate revert

# Or manually run down migration
psql $DATABASE_URL < migrations/<timestamp>_down.sql
```

**Important:** Database rollbacks are risky. Always:
- Test migrations in staging first
- Take database snapshot before production migrations
- Have tested rollback scripts ready

### Rollback Verification Checklist

After rollback:

- [ ] All tasks running successfully
- [ ] No error spikes in CloudWatch Logs
- [ ] Health check endpoints returning 200
- [ ] Database connections stable
- [ ] SQS message processing resumed
- [ ] End-to-end test passes
- [ ] Team notified of rollback completion

### Emergency Rollback

For critical issues requiring immediate action:

```bash
# Stop all tasks (halts processing)
aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --desired-count 0

# Roll back to known good version
aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --task-definition fossil-api:42 \
  --desired-count 2
```

## Security Considerations

### Network Security

1. **VPC Configuration**
   - Isolate services in private subnets
   - Use NAT Gateway for outbound internet access
   - Limit security group ingress to necessary ports only

```bash
# Security group for Fossil API (public-facing)
aws ec2 authorize-security-group-ingress \
  --group-id <sg-id> \
  --protocol tcp \
  --port 3000 \
  --cidr 0.0.0.0/0

# Security group for RDS (internal only)
aws ec2 authorize-security-group-ingress \
  --group-id <db-sg-id> \
  --protocol tcp \
  --port 5432 \
  --source-group <ecs-sg-id>
```

2. **Network ACLs**
   - Implement subnet-level network ACLs
   - Block known malicious IP ranges
   - Log denied connections

3. **VPC Flow Logs**
   ```bash
   aws ec2 create-flow-logs \
     --resource-type VPC \
     --resource-ids <vpc-id> \
     --traffic-type ALL \
     --log-destination-type cloud-watch-logs \
     --log-group-name /aws/vpc/fossil
   ```

### Secrets Management

1. **AWS Secrets Manager**

```bash
# Store secrets
aws secretsmanager create-secret \
  --name fossil/starknet-private-key \
  --secret-string "0xabcdef..." \
  --kms-key-id <kms-key-id>

# Enable automatic rotation (for database credentials)
aws secretsmanager rotate-secret \
  --secret-id fossil/db-password \
  --rotation-lambda-arn <lambda-arn> \
  --rotation-rules AutomaticallyAfterDays=30
```

2. **IAM Roles over Access Keys**

Use task-level IAM roles instead of hardcoded credentials:

```json
{
  "taskRoleArn": "arn:aws:iam::<account-id>:role/fossilTaskRole",
  "containerDefinitions": [
    {
      "environment": [
        {"name": "AWS_REGION", "value": "us-east-1"}
      ]
      // No AWS_ACCESS_KEY_ID or AWS_SECRET_ACCESS_KEY needed
    }
  ]
}
```

3. **Encryption at Rest**
   - Enable RDS encryption
   - Use encrypted EBS volumes
   - Encrypt secrets with KMS

4. **Never Commit Secrets**
   - Use `.gitignore` for environment files
   - Scan commits for leaked secrets (git-secrets, trufflehog)
   - Rotate any accidentally committed secrets immediately

### Access Control

1. **IAM Policies**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "sqs:SendMessage",
        "sqs:ReceiveMessage",
        "sqs:DeleteMessage"
      ],
      "Resource": "arn:aws:sqs:us-east-1:<account-id>:fossilQueue-prod"
    },
    {
      "Effect": "Allow",
      "Action": [
        "secretsmanager:GetSecretValue"
      ],
      "Resource": "arn:aws:secretsmanager:us-east-1:<account-id>:secret:fossil/*"
    }
  ]
}
```

2. **Database Security**
   - Use separate database users per service
   - Grant minimum required privileges
   - Enable SSL/TLS for database connections

```sql
-- Create read-only user for reporting
CREATE USER fossil_readonly WITH PASSWORD 'secure_password';
GRANT CONNECT ON DATABASE fossil_api TO fossil_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO fossil_readonly;
```

3. **API Authentication**
   - Implement API key authentication
   - Rate limiting per API key
   - Monitor for suspicious patterns

### Container Security

1. **Image Scanning**

```bash
# Scan images for vulnerabilities
aws ecr start-image-scan \
  --repository-name fossil-api \
  --image-id imageTag=latest

# Get scan results
aws ecr describe-image-scan-findings \
  --repository-name fossil-api \
  --image-id imageTag=latest
```

2. **Use Minimal Base Images**

```dockerfile
# Use distroless or Alpine for smaller attack surface
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/fossil-api /usr/local/bin/
CMD ["fossil-api"]
```

3. **Run as Non-Root User**

```dockerfile
RUN useradd -m -u 1000 fossil
USER fossil
```

### Compliance and Auditing

1. **Enable CloudTrail**

```bash
aws cloudtrail create-trail \
  --name fossil-audit-trail \
  --s3-bucket-name fossil-cloudtrail-logs \
  --is-multi-region-trail
```

2. **Regular Security Audits**
   - Review IAM permissions quarterly
   - Audit security group rules
   - Check for unused credentials
   - Scan dependencies for vulnerabilities

3. **AWS Config Rules**
   - Enforce encryption requirements
   - Check for public S3 buckets
   - Validate security group configurations

## Troubleshooting Deployment Issues

### ECS Task Launch Failures

**Problem:** Tasks fail to start

**Diagnosis:**

```bash
# Check stopped tasks
aws ecs list-tasks \
  --cluster fossil-cluster \
  --desired-status STOPPED \
  --max-items 5

# Get stop reason
aws ecs describe-tasks \
  --cluster fossil-cluster \
  --tasks <task-arn>
```

**Common Issues:**

1. **Insufficient Resources**
   - Error: "Cannot pull container image"
   - Solution: Check ECR permissions, ensure image exists

2. **Environment Variable Errors**
   - Error: "Essential container exited"
   - Solution: Check CloudWatch Logs for application errors

3. **Secrets Access Denied**
   - Error: "ResourceInitializationError"
   - Solution: Verify task execution role has Secrets Manager permissions

### Service Not Healthy

**Problem:** Health checks failing

**Diagnosis:**

```bash
# Check target group health
aws elbv2 describe-target-health \
  --target-group-arn <tg-arn>

# Check service events
aws ecs describe-services \
  --cluster fossil-cluster \
  --services fossil-api \
  --query 'services[0].events'
```

**Solutions:**

1. Check application logs for startup errors
2. Verify health check endpoint returns 200
3. Ensure security groups allow load balancer traffic
4. Increase health check grace period

### Database Connection Issues

**Problem:** Cannot connect to RDS

**Diagnosis:**

```bash
# Test from ECS task
aws ecs execute-command \
  --cluster fossil-cluster \
  --task <task-id> \
  --container fossil-api \
  --interactive \
  --command "/bin/sh"

# Inside container
nc -zv <rds-endpoint> 5432
```

**Solutions:**

1. Verify security group allows traffic from ECS tasks
2. Check VPC routing and subnet configuration
3. Verify database credentials in Secrets Manager
4. Ensure RDS instance is publicly accessible (if needed)

### High Memory/CPU Usage

**Problem:** Tasks being killed due to resource exhaustion

**Diagnosis:**

```bash
# Check CloudWatch metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/ECS \
  --metric-name MemoryUtilization \
  --dimensions Name=ServiceName,Value=message-handler Name=ClusterName,Value=fossil-cluster \
  --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%S) \
  --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
  --period 300 \
  --statistics Average
```

**Solutions:**

1. Increase task CPU/memory allocation
2. Enable auto-scaling based on utilization
3. Optimize application code (profiling, memory leaks)
4. Scale horizontally (more tasks) instead of vertically

### Deployment Stuck

**Problem:** Deployment not progressing

**Diagnosis:**

```bash
aws ecs describe-services \
  --cluster fossil-cluster \
  --services fossil-api \
  --query 'services[0].deployments'
```

**Solutions:**

1. Check if old tasks are draining properly
2. Verify health checks are passing
3. Increase deployment circuit breaker failure threshold
4. Force new deployment:

```bash
aws ecs update-service \
  --cluster fossil-cluster \
  --service fossil-api \
  --force-new-deployment
```

### SQS Message Processing Issues

**Problem:** Messages not being processed

**Diagnosis:**

```bash
# Check queue metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/SQS \
  --metric-name ApproximateNumberOfMessagesVisible \
  --dimensions Name=QueueName,Value=fossilQueue-prod \
  --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%S) \
  --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
  --period 300 \
  --statistics Average

# Check message handler logs
aws logs filter-log-events \
  --log-group-name /ecs/message-handler \
  --filter-pattern "ERROR"
```

**Solutions:**

1. Verify message handler tasks are running
2. Check AWS credentials for SQS access
3. Verify SQS_QUEUE_URL is correct
4. Increase message visibility timeout if processing takes long
5. Check for dead letter queue messages

### Docker Image Build Failures

**Problem:** CI/CD build fails

**Solutions:**

1. Check Dockerfile syntax
2. Verify all dependencies are available
3. Increase Docker build memory limit
4. Use multi-stage builds to reduce image size
5. Cache intermediate layers in CI/CD

### Network Timeout Issues

**Problem:** Services timing out when communicating

**Diagnosis:**

```bash
# Check VPC endpoint connectivity
aws ecs execute-command \
  --cluster fossil-cluster \
  --task <task-id> \
  --container fossil-api \
  --interactive \
  --command "/bin/sh"

# Test connectivity
curl -v http://proving-service-api:3001/health
```

**Solutions:**

1. Verify service discovery is configured
2. Check security group rules allow inter-service traffic
3. Increase application timeout settings
4. Verify NAT gateway and internet gateway configuration

## Related Documentation

- [Environment Setup Guide](environment-setup.md) - Detailed environment variable configuration
- [Local Development Guide](../getting-started/local-development.md) - Setting up local environment
- [Database Management Guide](database-management.md) - Database setup and migrations
- [Architecture Overview](../architecture/overview.md) - System architecture details
- [API Reference](../api-reference/) - API endpoint documentation

## Additional Resources

- [AWS ECS Documentation](https://docs.aws.amazon.com/ecs/)
- [Docker Best Practices](https://docs.docker.com/develop/dev-best-practices/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [StarkNet Documentation](https://docs.starknet.io/)
- [RISC Zero Documentation](https://dev.risczero.com/)
