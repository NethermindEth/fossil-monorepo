use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(sqlx::FromRow, Debug)]
pub struct ApiKey {
    pub key: String,
    pub name: Option<String>,
}

#[derive(sqlx::Type, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[sqlx(type_name = "TEXT")]
pub enum JobStatus {
    Pending,
    Completed,
    Failed,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L1Data {
    pub twap: String,          // u256 as hex string
    pub max_return: String,    // u128 as hex string
    pub reserve_price: String, // u256 as hex string
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnChainConfirmation {
    pub block_number: u64,
    pub transaction_hash: String,
    pub event_timestamp: u64,
}

#[derive(sqlx::FromRow, Debug)]
pub struct JobRequest {
    pub job_id: String,
    pub status: JobStatus,
    pub vault_address: Option<String>,
    pub expected_timestamp: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub result: Option<serde_json::Value>,
    pub l1_data: Option<serde_json::Value>,
    pub on_chain_confirmation: Option<serde_json::Value>,
}
