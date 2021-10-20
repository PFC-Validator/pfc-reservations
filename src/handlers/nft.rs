use crate::auth::{verify_signature, SignatureB64};
use crate::requests::NewNFTResponse;
use crate::NFTDatabase;
use crate::{requests, ReservationState};

use pfc_reservation::requests::{ErrorResponse, NFTTallyResponse};
use postgres::Statement;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde_json::Value;
use uuid::Uuid;

use crate::models::NFT;

/// returns the status of the NFTs
#[get("/")]
async fn index(
    conn: NFTDatabase,
) -> (
    Status,
    Result<Json<Vec<NFTTallyResponse>>, Json<ErrorResponse>>,
) {
    match conn
        .run(move |c| {
            c.query(
                "select assigned, reserved, count(*) from nft group by  assigned, reserved",
                &[],
            )
        })
        .await
    {
        Ok(rows) => {
            let tally: Vec<NFTTallyResponse> = rows
                .iter()
                .map(|row| NFTTallyResponse {
                    assigned: row.get(0),
                    reserved: row.get(1),
                    count: row.get(2),
                })
                .collect();
            (Status::new(200), Ok(Json(tally)))
        }
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: format!("DB Error:{})", e),
            })),
        ),
    }
}

#[get("/<id>")]
async fn get_by_id(
    conn: NFTDatabase,
    id: Uuid,
) -> (Status, Result<Json<NFT>, Json<ErrorResponse>>) {
    //    let uuid_id: Uuid = Uuid::from_str(&id).unwrap();
    match conn
        .run(move |c| c.query("Select id,name from NFT where id=$1", &[&id]))
        .await
    {
        Ok(results) => {
            for row in results {
                let nft = NFT {
                    id: row.get(0),
                    name: row.get(1),
                };
                return (Status::new(200), Ok(Json(nft)));
            }
            (
                Status::new(404),
                Err(Json(ErrorResponse {
                    code: 404,
                    message: "Not Found".into(),
                })),
            )
        }
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: format!("DB Error:{})", e),
            })),
        ),
    }
}

#[post("/new", format = "json", data = "<nft_in>")]
async fn new_nft(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    nft_in: Json<requests::NewNFTRequest>,
) -> (Status, Result<Json<NewNFTResponse>, Json<ErrorResponse>>) {
    // log::info!("{}", signature.signature);
    let nft_in_stuff = nft_in.into_inner();
    let nft_in_json = serde_json::to_string(&nft_in_stuff).unwrap();
    match verify_signature(&nft_in_json, &signature, &state.verification_key) {
        Ok(()) => {
            let meta_json: Value = serde_json::from_str(&nft_in_stuff.meta).unwrap();
            let svg_json: Value = serde_json::from_str(&nft_in_stuff.svg).unwrap();
            match conn
                .run(move |c| {
                    let stmt: Statement = c
                        .prepare(
                            r#"Insert into NFT( id,name,meta_data,svg,ipfs_image,
                                                ipfs_meta, image_data, external_url,
                                                description,background_color,
                                                animation_url,youtube_url  )     
                            values(DEFAULT,$1,$2,$3,$4, $5,$6,$7, $8,$9, $10,$11) returning id"#,
                        )
                        .unwrap();
                    c.query(
                        &stmt,
                        &[
                            &nft_in_stuff.name,
                            &meta_json,
                            &svg_json,
                            &nft_in_stuff.ipfs_image,
                            &nft_in_stuff.ipfs_meta,
                            &nft_in_stuff.image_data,
                            &nft_in_stuff.external_url,
                            &nft_in_stuff.description,
                            &nft_in_stuff.background_color,
                            &nft_in_stuff.animation_url,
                            &nft_in_stuff.youtube_url,
                        ],
                    )
                })
                .await
            {
                Ok(new_nft) => {
                    let row = new_nft.first();
                    let id_returned: Uuid = row.unwrap().get(0);
                    log::info!("{:?}", id_returned);
                    let response = NewNFTResponse {
                        nft_id: id_returned,
                    };
                    (Status::new(201), Ok(Json(response)))
                }
                Err(db_err) => (
                    Status::new(500),
                    Err(Json(ErrorResponse {
                        code: 500,
                        message: db_err.to_string(),
                    })),
                ),
            }
        }
        Err(e) => (
            Status::new(403),
            Err(Json(ErrorResponse {
                code: 403,
                message: e.to_string(),
            })),
        ),
    }
}

pub fn get_routes() -> Vec<Route> {
    routes![index, get_by_id, new_nft]
}
