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
pub struct NameNFTResponse {
    pub allowed: bool,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct NFTTallyResponse {
    pub assigned: i64,
    pub reserved: i64,
    pub in_process: i64,
    pub available: i64,
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
    pub in_process: bool,
    pub tx_hash: Option<String>,
    pub tx_error: Option<String>,
    pub tx_retry_count: i32,
    pub token_id: Option<String>,
}

/// request a NFT to be reserved
#[derive(Serialize, Deserialize, Clone)]
pub struct NewReservationRequest {
    /// wallet requesting reservation
    pub wallet_address: String,
    /// how long to hold the reservation
    pub reserved_until: DateTime<Utc>,
    // optionally,
    //pub stage: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NewReservationResponse {
    pub nft_id: Uuid,
    pub metadata_response: MetadataResponse,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Metadata {
    pub token_uri: String,
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MetadataResponse {
    pub attributes: String,
    pub signature: String,
}

/// submit TX hash of NFT assignment.
#[derive(Serialize, Deserialize, Clone)]
pub struct AssignHashRequest {
    /// wallet requesting reservation
    pub wallet_address: String,
    /// NFT id
    pub nft_id: Uuid,
    /// the hash for the request
    pub tx_hash: String,
}
/// submit signed request to perform NFT assignment
#[derive(Serialize, Deserialize, Clone)]
pub struct AssignSignedTxRequest {
    /// wallet requesting reservation
    pub wallet_address: String,
    /// NFT id
    pub nft_id: Uuid,
    /// the hash for the request
    pub signed_tx: String,
}
/// submit signed request to process TX result
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReservationTxResultRequest {
    /// the hash for the request
    pub tx: String,
    pub wallet_address: Option<String>,
    pub assigned_on: Option<DateTime<Utc>>,
    pub token_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

pub struct OpenStageWallet {
    pub wallet_address: String,
    pub stage_id: Uuid,
    pub allocated: i32,
    pub reserved: i32,
    pub assigned: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MintReservation {
    /// wallet requesting reservation
    pub wallet_address: String,
    /// the 'id' of the NFT in our system
    pub nft_id: Uuid,
    /// the meta data to mint it
    pub meta_data: Metadata,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AssignOwner {
    pub token_id: String,
    pub wallet_address: String,
}
