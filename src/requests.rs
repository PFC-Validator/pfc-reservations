use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct NewNFTRequest {
    pub name: String,
    pub meta: String,
    pub svg: String,
}
#[derive(Serialize)]
pub struct NewNFTResponse {
    pub nft_id: Uuid,
}
