use chrono::DateTime;
//use rocket_sync_db_pools::diesel::Queryable;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize)]
pub struct NFT {
    pub id: Uuid,
    pub name: String,
    pub assigned: bool,
    pub reserved: bool,
    pub has_submit_error: bool,
    pub reserved_until: Option<DateTime<chrono::offset::Utc>>,
    pub in_process: bool,
    pub txhash: Option<String>,
}
#[derive(Serialize)]
pub struct NftFull {
    pub nft_lite: NFT,
    pub meta_data: Value,
    pub svg: Value,
    pub ipfs_image: String,
    pub ipfs_meta: String,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    pub assigned_on: Option<DateTime<chrono::offset::Utc>>,
    pub assigned_to_wallet_address: Option<String>,
    pub reserved_to_wallet_address: Option<String>,
    pub signed_packet: Option<Value>,
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
