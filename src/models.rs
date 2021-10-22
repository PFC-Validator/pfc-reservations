use chrono::DateTime;
//use rocket_sync_db_pools::diesel::Queryable;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct NFT {
    pub id: Uuid,
    pub name: String,
    pub assigned: bool,
    pub reserved: bool,
    pub has_submit_error: bool,
    pub reserved_until: Option<DateTime<chrono::offset::Utc>>,
}

#[derive(Serialize)]
pub struct Stage {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub attribute_type: Option<String>,
    pub attribute_value: Option<String>,
    pub is_default: bool,
    pub stage_free: bool,
    pub stage_open: DateTime<chrono::offset::Utc>,
}

#[derive(Serialize)]
pub(crate) struct WalletStageAllocation {
    pub id: Option<Uuid>,
    pub allocation_count: i64,
    pub reserved_count: i64,
    pub assigned_count: i64,
    pub stage_open: Option<DateTime<chrono::offset::Utc>>,
}
