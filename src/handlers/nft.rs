use crate::auth::{verify_signature, SignatureB64};
use crate::requests::NewNFTResponse;
use crate::NFTDatabase;
use crate::{requests, ReservationState};
use chrono::{DateTime, Utc};

use crate::db::{get_nft_stat, get_stages};
use pfc_reservation::requests::{ErrorResponse, NFTStageTallyStat, NFTTallyResponse, NFTTallyStat};
use postgres::Statement;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde_json::Value;
use uuid::Uuid;

use crate::models::NFT;

/// returns the status of the NFTs
#[get("/")]
async fn index(conn: NFTDatabase) -> (Status, Result<Json<NFTTallyResponse>, Json<ErrorResponse>>) {
    match conn
        .run(move |c| {
            c.query_one(
               r#"
               select  sum(case assigned when true then 1 else 0 end) as assigned,
                       sum(case reserved and reserved_until > now() and not assigned and not in_process when true then 1 else 0 end) as reserved,
                       sum(case in_process when true then 1 else 0 end ) as in_process,
                       sum(case not assigned and not in_process and (not reserved or reserved_until <= now()) when true then 1 else 0 end) as available
               from nft"#,
                &[],
            )
        })
        .await
    {
        Ok(row) => {
            let tally =NFTTallyResponse {
                assigned: row.get(0),
                reserved: row.get(1),
                in_process: row.get(2),
                available: row.get(3)
            };

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

#[get("/stages")]
async fn get_stage_stats(
    conn: NFTDatabase,
) -> (
    Status,
    Result<Json<Vec<NFTStageTallyStat>>, Json<ErrorResponse>>,
) {
    conn.run(move |mut c| match get_stages(&mut c) {
        Ok(stages) => {
            let stats = stages.iter().map(|s| {
                let st = match get_nft_stat(c, &s.attribute_type, &s.attribute_value) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Get State Stats: {}", e.1.message);
                        NFTTallyStat {
                            assigned: -1,
                            reserved: -1,
                            count: -1,
                        }
                    }
                };

                NFTStageTallyStat {
                    stage_id: s.id,
                    stage_code: s.code.clone(),
                    stage_name: s.name.clone(),
                    wallet_count: -1,
                    stats: st,
                }
            });
            (Status::new(200), Ok(Json(stats.collect::<Vec<_>>())))
        }
        Err(e) => (e.0, Err(e.1)),
    })
    .await
}
#[get("/<id>")]
async fn get_by_id(
    conn: NFTDatabase,
    id: Uuid,
) -> (Status, Result<Json<NFT>, Json<ErrorResponse>>) {
    //    let uuid_id: Uuid = Uuid::from_str(&id).unwrap();
    match conn
        .run(move |c| c.query("Select id, name, assigned, reserved, has_submit_error,reserved_until, in_process from NFT where id=$1", &[&id]))
        .await
    {
        Ok(results) => {
            if let Some(row) = results.first() {
                let now: DateTime<chrono::offset::Utc> = Utc::now();
                let assigned:bool = row.get(2);
                let in_process:bool = row.get(6);
                if  assigned || in_process {
                     (Status::new(200), Ok(Json(NFT {
                        id: row.get(0),
                        name: row.get(1),
                        assigned: row.get(2),
                        reserved: row.get(3),
                        has_submit_error: row.get(4),
                        reserved_until: row.get(5),
                        in_process: row.get(6),
                        txhash: Some(String::from("-hidden-"))
                    })))
                } else {
                    let reserved_date_o: Option<DateTime<chrono::offset::Utc>> = row.get(5);
                    if let Some(reserved_date) = reserved_date_o {
                        if now < reserved_date {
                         //   log::info!("In Reservation {} {}",now, reserved_date);
                            (Status::new(200), Ok(Json(NFT {
                                id: row.get(0),
                                name: row.get(1),
                                assigned: row.get(2),
                                reserved: true,
                                has_submit_error: row.get(4),
                                reserved_until: Some(reserved_date),
                                in_process: row.get(6),
                                txhash: None
                            })))
                        } else {
                            log::info!("Past Reservation {} {}",now, reserved_date);
                            (Status::new(200), Ok(Json(NFT {
                                id: row.get(0),
                                name: row.get(1),
                                assigned: row.get(2),
                                reserved: false,
                                has_submit_error: row.get(4),
                                reserved_until: None,
                                in_process: row.get(6),
                                txhash: None
                            })))
                        }
                    } else {
                         (Status::new(200), Ok(Json(NFT {
                            id: row.get(0),
                            name: row.get(1),
                            assigned: row.get(2),
                            reserved: false,
                            has_submit_error: row.get(4),
                            reserved_until: None,
                            in_process: row.get(6),
                            txhash: Some(String::from("-hidden-"))
                        })))
                    }
                }
            } else {
                 (Status::new(404), Err(Json(ErrorResponse {
                    code: 404,
                    message: String::from("NFT not found"),
                })))
            }
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
    routes![index, get_by_id, new_nft, get_stage_stats]
}
