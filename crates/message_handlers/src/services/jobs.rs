use risc0_zkvm::Receipt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestProof {
    pub job_id: String,
    pub job_group_id: Option<String>,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofGenerated {
    pub job_id: String,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Job {
    RequestProof(RequestProof),
    ProofGenerated(Box<ProofGenerated>),
}
