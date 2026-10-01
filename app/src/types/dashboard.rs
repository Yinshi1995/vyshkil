use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashboardStats {
    pub org_count: i64,
    pub training_group_count: i64,
    pub committed_submission_count: i64,
    pub draft_submission_count: i64,
    pub open_discrepancy_count: i64,
    pub notification_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentSubmission {
    pub id: i32,
    pub org_label: String,
    pub source_type: String,
    pub status: String,
    pub updated_at: String,
}
