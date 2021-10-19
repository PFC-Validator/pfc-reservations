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
pub struct Reservation {
    pub wallet_address: String,
    pub nft_id: Uuid,
    pub reserved: bool,
    pub reserved_until: Option<DateTime<Utc>>,
    pub assigned: bool,
    pub assigned_on: Option<DateTime<Utc>>,
    pub has_submit_error: bool,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct NewReservationRequest {
    pub wallet_address: String,
    pub reserved_until: DateTime<Utc>,
    pub signed_tx: Option<String>,
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
