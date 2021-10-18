//use rocket_sync_db_pools::diesel::Queryable;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct NFT {
    pub id: Uuid,
    pub name: String,
}
