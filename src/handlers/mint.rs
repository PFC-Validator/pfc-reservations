use crate::auth::{generate_signature, is_valid_address, verify_signature, SignatureB64};
use crate::db::{
    get_nft, nft_assign_owner, nft_assign_tx_result, set_tx_for_nft, set_tx_hash_for_nft,
};
use crate::models::NFT;
use crate::requests::{
    AssignHashRequest, AssignOwner, AssignSignedTxRequest, ErrorResponse, Metadata,
    MetadataResponse, NewReservationResponse, ReservationTxResultRequest,
};
use crate::{NFTDatabase, ReservationState};
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};
use serde_json::Value;
use terra_rust_api::PrivateKey;
use uuid::Uuid;

/// Ensures that NFT is reserved, and the reservation has not expired
///
fn validate_reservation(nft: &NFT) -> (Status, Result<bool, Json<ErrorResponse>>) {
    if !nft.reserved {
        (
            Status::new(401),
            Err(Json(ErrorResponse {
                code: 401,
                message: String::from("Not Reserved"),
            })),
        )
    } else if nft.has_submit_error {
        log::info!(
            "{} - Reservation with error being retried - {:?}",
            nft.id,
            nft.reserved_until
        );
        (Status::new(200), Ok(true))
    } else if nft.reserved_until.is_none() || nft.reserved_until.unwrap() < Utc::now() {
        (
            Status::new(401),
            Err(Json(ErrorResponse {
                code: 401,
                message: String::from("Reservation has expired"),
            })),
        )
    } else {
        (Status::new(200), Ok(true))
    }
}
/// returns metadata for a given NFT, and a signature of it. with the wallet address embedded.
///  let hash_message = format!("{}/{}", info.sender, msg.attributes);
pub fn build_metadata_response(
    wallet_address: &str,
    signing_key: &PrivateKey,
    nft_meta: &Value,
) -> (Status, Result<MetadataResponse, Json<ErrorResponse>>) {
    //let meta = &nft.meta_data;
    match serde_json::from_value::<Metadata>(nft_meta.clone()) {
        Ok(m) => {
            let attributes = serde_json::to_string(&m).unwrap();
            let to_sign = format!("{}/{}", wallet_address, attributes);

            match generate_signature(signing_key, &to_sign) {
                Ok(sig) => (
                    Status::new(200),
                    Ok(MetadataResponse {
                        attributes,
                        signature: sig.signature,
                    }),
                ),
                Err(e) => (Status::new(500), Err(e)),
            }
        }
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}
#[options("/<_wallet>/<_nft>")]
async fn options_signed_metadata(
    _wallet: String,
    _nft: Uuid,
) -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}

#[get("/<wallet>/<nft>")]
async fn get_signed_metadata(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    wallet: String,
    nft: Uuid,
) -> (
    Status,
    Result<Json<NewReservationResponse>, Json<ErrorResponse>>,
) {
    if let Err(e) = is_valid_address(&wallet) {
        return (Status::new(403), Err(e));
    }
    let ss = format!("{{\"nft\":\"{}\"}}", nft.to_string());

    //  log::info!("{}", ss);
    if let Err(e) = verify_signature(&ss, &signature, &state.verification_key) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    let signing_key = &state.signing_key;

    let nft_r = conn.run(move |c| get_nft(c, &nft)).await;
    match nft_r {
        Ok(nft_full) => {
            if let Some(reserved_to) = &nft_full.reserved_to_wallet_address {
                if reserved_to.eq(&wallet) {
                    let result = validate_reservation(&nft_full.nft_lite);
                    match result.1 {
                        Ok(_) => {
                            let x = build_metadata_response(
                                reserved_to,
                                signing_key,
                                &nft_full.meta_data,
                            );
                            match x.1 {
                                Ok(y) => (
                                    x.0,
                                    Ok(Json(NewReservationResponse {
                                        nft_id: nft,
                                        metadata_response: y,
                                    })),
                                ),
                                Err(e) => (x.0, Err(e)),
                            }
                        }
                        Err(e) => (result.0, Err(e)),
                    }
                } else {
                    (
                        Status::new(401),
                        Err(Json(ErrorResponse {
                            code: 401,
                            message: String::from("Invalid Reservation"),
                        })),
                    )
                }
            } else {
                (
                    Status::new(500),
                    Err(Json(ErrorResponse {
                        code: 500,
                        message: String::from("NFT is not reserved"),
                    })),
                )
            }
        }
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}

#[options("/hash")]
async fn options_assign_txhash() -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}
#[post("/hash", format = "json", data = "<assign_hash_request>")]
async fn assign_txhash(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    assign_hash_request: Json<AssignHashRequest>,
) -> (Status, Result<Json<bool>, Json<ErrorResponse>>) {
    let assign_hash_request_stuff = assign_hash_request.into_inner();
    let assign_hash_request_json = serde_json::to_string(&assign_hash_request_stuff).unwrap();

    if let Err(e) = verify_signature(
        &assign_hash_request_json,
        &signature,
        &state.verification_key,
    ) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    let nft_id = assign_hash_request_stuff.nft_id;
    conn.run(move |c| {
        let nft_r = get_nft(c, &nft_id);

        match nft_r {
            Ok(nft_full) => {
                if let Some(reserved_to) = &nft_full.reserved_to_wallet_address {
                    if reserved_to.eq(&assign_hash_request_stuff.wallet_address)
                        && nft_full.nft_lite.reserved
                    {
                        match set_tx_hash_for_nft(c, &nft_id, &assign_hash_request_stuff.tx_hash) {
                            Ok(n) => (Status::new(200), Ok(Json(n == 1))),
                            Err(e) => (
                                Status::new(500),
                                Err(Json(ErrorResponse {
                                    message: e.to_string(),
                                    code: 500,
                                })),
                            ),
                        }
                    } else {
                        (
                            Status::new(500),
                            Err(Json(ErrorResponse {
                                code: 500,
                                message: String::from("NFT is not reserved to wallet"),
                            })),
                        )
                    }
                } else {
                    (
                        Status::new(500),
                        Err(Json(ErrorResponse {
                            code: 500,
                            message: String::from("NFT is not reserved"),
                        })),
                    )
                }
            }
            Err(e) => (
                Status::new(500),
                Err(Json(ErrorResponse {
                    code: 500,
                    message: e.to_string(),
                })),
            ),
        }
    })
    .await
}

#[options("/tx")]
async fn options_assign_tx() -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}
#[post("/tx", format = "json", data = "<assign_hash_request>")]
async fn assign_tx(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    assign_hash_request: Json<AssignSignedTxRequest>,
) -> (Status, Result<Json<bool>, Json<ErrorResponse>>) {
    let assign_hash_request_stuff = assign_hash_request.into_inner();
    let assign_hash_request_json = serde_json::to_string(&assign_hash_request_stuff).unwrap();

    if let Err(e) = verify_signature(
        &assign_hash_request_json,
        &signature,
        &state.verification_key,
    ) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    let nft_id = assign_hash_request_stuff.nft_id;
    conn.run(move |c| {
        let nft_r = get_nft(c, &nft_id);

        match nft_r {
            Ok(nft_full) => {
                if let Some(reserved_to) = &nft_full.reserved_to_wallet_address {
                    if reserved_to.eq(&assign_hash_request_stuff.wallet_address)
                        && nft_full.nft_lite.reserved
                    {
                        match set_tx_for_nft(c, &nft_id, &assign_hash_request_stuff.signed_tx) {
                            Ok(n) => (Status::new(200), Ok(Json(n == 1))),
                            Err(e) => (
                                Status::new(500),
                                Err(Json(ErrorResponse {
                                    message: e.to_string(),
                                    code: 500,
                                })),
                            ),
                        }
                    } else {
                        (
                            Status::new(500),
                            Err(Json(ErrorResponse {
                                code: 500,
                                message: String::from("NFT is not reserved to wallet"),
                            })),
                        )
                    }
                } else {
                    (
                        Status::new(500),
                        Err(Json(ErrorResponse {
                            code: 500,
                            message: String::from("NFT is not reserved"),
                        })),
                    )
                }
            }
            Err(e) => (
                Status::new(500),
                Err(Json(ErrorResponse {
                    code: 500,
                    message: e.to_string(),
                })),
            ),
        }
    })
    .await
}
#[options("/tx_result")]
async fn options_assign_tx_result() -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}
#[post("/tx_result", format = "json", data = "<hash_result>")]
async fn assign_tx_result(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    hash_result: Json<ReservationTxResultRequest>,
) -> (Status, Result<Json<bool>, Json<ErrorResponse>>) {
    let hash_result_stuff = hash_result.into_inner();
    let hash_result_stuff_json = serde_json::to_string(&hash_result_stuff).unwrap();
    log::info!("hash_result:{}", hash_result_stuff_json);
    if let Err(e) = verify_signature(&hash_result_stuff_json, &signature, &state.verification_key) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    //  let tx = hash_result_stuff.tx;
    conn.run(move |c| {
        let assign_result = nft_assign_tx_result(
            c,
            hash_result_stuff.wallet_address,
            hash_result_stuff.tx,
            hash_result_stuff.success,
            hash_result_stuff.assigned_on,
            hash_result_stuff.error,
            hash_result_stuff.token_id,
        );

        match assign_result {
            Ok(rows_updated) => {
                if rows_updated == 1 {
                    (Status::new(200), Ok(Json(true)))
                } else {
                    (
                        Status::new(500),
                        Err(Json(ErrorResponse {
                            code: 500,
                            message: String::from("TX not found"),
                        })),
                    )
                }
            }
            Err(e) => (
                Status::new(500),
                Err(Json(ErrorResponse {
                    code: 500,
                    message: e.to_string(),
                })),
            ),
        }
    })
    .await
}

#[options("/assign-owner")]
async fn options_assign_owner() -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}
#[post("/assign-owner", format = "json", data = "<assign_owner>")]
async fn assign_owner(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    assign_owner: Json<AssignOwner>,
) -> (Status, Result<Json<bool>, Json<ErrorResponse>>) {
    let assign_owner_stuff = assign_owner.into_inner();
    let assign_owner_stuff_json = serde_json::to_string(&assign_owner_stuff).unwrap();
    log::debug!("assign_assign_owner:{}", assign_owner_stuff_json);
    if let Err(e) = verify_signature(
        &assign_owner_stuff_json,
        &signature,
        &state.verification_key,
    ) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    //  let tx = hash_result_stuff.tx;
    conn.run(move |c| {
        let assign_result = nft_assign_owner(
            c,
            assign_owner_stuff.wallet_address,
            assign_owner_stuff.token_id,
        );

        match assign_result {
            Ok(rows_updated) => {
                if rows_updated == 1 {
                    (Status::new(200), Ok(Json(true)))
                } else {
                    (
                        Status::new(500),
                        Err(Json(ErrorResponse {
                            code: 500,
                            message: String::from("token/wallet not found"),
                        })),
                    )
                }
            }
            Err(e) => (
                Status::new(500),
                Err(Json(ErrorResponse {
                    code: 500,
                    message: e.to_string(),
                })),
            ),
        }
    })
    .await
}

pub fn get_routes() -> Vec<Route> {
    routes![
        get_signed_metadata,
        assign_txhash,
        assign_tx,
        assign_tx_result,
        assign_owner,
        options_assign_txhash,
        options_assign_tx,
        options_signed_metadata,
        options_assign_tx_result,
        options_assign_owner
    ]
}
