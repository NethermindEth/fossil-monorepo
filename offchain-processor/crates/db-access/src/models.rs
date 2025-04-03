use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(sqlx::FromRow, Debug)]
pub struct ApiKey {
    pub key: String,
    pub name: Option<String>,
}

#[derive(sqlx::Type, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(sqlx::FromRow, Debug)]
pub struct JobRequest {
    pub job_id: String,
    pub status: JobStatus,
    pub created_at: chrono::NaiveDateTime,
    pub result: Option<serde_json::Value>,
}
