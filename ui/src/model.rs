use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub created_at: DateTime<Utc>,
}
