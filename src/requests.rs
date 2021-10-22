use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct NewNFTRequest {
    pub name: String,
    pub meta: String,
    pub svg: String,
    pub ipfs_image: String,
    pub ipfs_meta: String,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}
#[derive(Serialize)]
pub struct NewNFTResponse {
    pub nft_id: Uuid,
}

#[derive(Serialize)]
pub struct NFTTallyResponse {
    pub assigned: bool,
    pub reserved: bool,
    pub count: i64,
}
#[derive(Serialize)]
pub struct NFTTallyStat {
    pub assigned: i64,
    pub reserved: i64,
    pub count: i64,
}
#[derive(Serialize)]
pub struct NFTStageTallyStat {
    pub stage_id: Uuid,
    pub stage_code: String,
    pub stage_name: String,
    pub wallet_count: i64,
    pub stats: NFTTallyStat,
}

#[derive(Serialize)]
pub struct Reservation {
    pub wallet_address: String,
    pub nft_id: Uuid,
    pub reserved: bool,
    pub reserved_until: Option<DateTime<Utc>>,
    pub assigned: bool,
    pub assigned_on: Option<DateTime<Utc>>,
    pub has_submit_error: bool,
}

/// request a NFT to be reserved
#[derive(Serialize, Deserialize, Clone)]
pub struct NewReservationRequest {
    /// wallet requesting reservation
    pub wallet_address: String,
    /// how long to hold the reservation
    pub reserved_until: DateTime<Utc>,
    /// optionally,
    pub stage: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NewReservationResponse {
    pub nft_id: Uuid,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
}
