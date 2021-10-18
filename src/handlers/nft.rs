use crate::auth::{verify_signature, SignatureB64};
use crate::requests::NewNFTResponse;
use crate::NFTDatabase;
use crate::{requests, ReservationState};
use postgres::Statement;
use rocket::response::status::Created;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde_json::Value;
use uuid::Uuid;

use crate::models::NFT;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}
#[get("/<id>")]
async fn get_by_id(conn: NFTDatabase, id: Uuid) -> Option<Json<NFT>> {
    //    let uuid_id: Uuid = Uuid::from_str(&id).unwrap();
    let results = conn
        .run(move |c| c.query("Select id,name from NFT where id=$1", &[&id]))
        .await
        .unwrap();
    for row in results {
        let nft = NFT {
            id: row.get(0),
            name: row.get(1),
        };
        return Some(Json(nft));
    }
    None
}

#[post("/new", format = "json", data = "<nft_in>")]
async fn new_nft(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    nft_in: Json<requests::NewNFTRequest>,
) -> Created<Json<requests::NewNFTResponse>> {
    log::info!("{}", signature.signature);
    let nft_in_stuff = nft_in.into_inner();
    let nft_in_json = serde_json::to_string(&nft_in_stuff).unwrap();
    verify_signature(&nft_in_json, &signature, &state.verification_key).unwrap();
    let meta_json: Value = serde_json::from_str(&nft_in_stuff.meta).unwrap();
    let svg_json: Value = serde_json::from_str(&nft_in_stuff.svg).unwrap();
    let new_nft: Vec<postgres::Row> = conn
        .run(move |c| {
            let stmt: Statement = c
                .prepare(
                    "Insert into NFT(id,name,meta_data,svg) values(DEFAULT,$1,$2,$3) returning id",
                )
                .unwrap();
            c.query(&stmt, &[&nft_in_stuff.name, &meta_json, &svg_json])
        })
        .await
        .unwrap();

    let row = new_nft.first();
    let id_returned: Uuid = row.unwrap().get(0);
    log::info!("{:?}", id_returned);
    let response = NewNFTResponse {
        nft_id: id_returned,
    };
    Created::new("/new").body(Json(response))
}

pub fn get_routes() -> Vec<Route> {
    routes![index, get_by_id, new_nft]
}
