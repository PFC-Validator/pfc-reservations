use crate::auth::{is_valid_address, verify_signature, SignatureB64};
use crate::db::{
    do_reservation, get_open_wallets_for_stage, get_reservations_for_wallet, get_stage,
    mint_nft_for_wallet_in_stage, reservations_in_mint_process, reservations_in_mint_reserved,
    reservations_stuck_in_mint_process,
};
use crate::handlers::mint::build_metadata_response;
use crate::requests::{ErrorResponse, NewReservationRequest, NewReservationResponse, Reservation};
use crate::{NFTDatabase, ReservationState};
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::db::reservations_in_process;

use crate::requests::MintReservation;
use uuid::Uuid;

#[get("/<address>")]
async fn get_by_address(
    conn: NFTDatabase,
    address: String,
) -> (Status, Result<Json<Vec<Reservation>>, Json<ErrorResponse>>) {
    if let Err(e) = is_valid_address(&address) {
        return (Status::new(403), Err(e));
    }
    conn.run(move |c| get_reservations_for_wallet(c, &address))
        .await
}

#[options("/new")]
async fn options_new_reservation() -> rocket::response::status::Custom<String> {
    rocket::response::status::Custom(Status::new(200), "OK".into())
}

#[post("/new", format = "json", data = "<reservation_in>")]
async fn new_reservation(
    conn: NFTDatabase,
    signature: SignatureB64,
    state: &State<ReservationState>,
    reservation_in: Json<NewReservationRequest>,
) -> (
    Status,
    Result<Json<NewReservationResponse>, Json<ErrorResponse>>,
) {
    let reservation_in_stuff = reservation_in.into_inner();
    let reservation_in_json = serde_json::to_string(&reservation_in_stuff).unwrap();

    if let Err(e) = verify_signature(&reservation_in_json, &signature, &state.verification_key) {
        if state.debug_mode {
            log::warn!("IGNORING SIGNATURES");
        } else {
            log::warn!(
                "Signature Failed {}/{}",
                reservation_in_json,
                &state.verification_key.join(",")
            );
            return (
                Status::new(403),
                Err(Json(ErrorResponse {
                    code: 403,
                    message: e.to_string(),
                })),
            );
        }
    }
    let duration_max = Utc::now() + state.max_reservation_duration;
    if reservation_in_stuff.reserved_until.gt(&duration_max) {
        return (
            Status::new(401),
            Err(Json(ErrorResponse {
                code: 401,
                message: "Exceeds maximum reservation length".to_string(),
            })),
        );
    }
    if reservation_in_stuff.reserved_until.lt(&Utc::now()) {
        return (
            Status::new(401),
            Err(Json(ErrorResponse {
                code: 401,
                message: "reservation time has already expired".to_string(),
            })),
        );
    }
    if let Err(f) = is_valid_address(&reservation_in_stuff.wallet_address.clone()) {
        return (Status::new(401), Err(f));
    }

    let max_reservations = state.max_reservations;
    let wallet_address = reservation_in_stuff.wallet_address.clone();
    let reserved_until = reservation_in_stuff.reserved_until;
    let result: (
        Status,
        Result<(Uuid, serde_json::Value), Json<ErrorResponse>>,
    ) = conn
        .run(move |c| {
            do_reservation(
                c,
                &wallet_address,
                &reserved_until.clone(),
                max_reservations,
            )
        })
        .await;
    match result.1 {
        Ok((uuid, meta_data)) => {
            let signing_key = &state.signing_key;
            let att = build_metadata_response(
                &reservation_in_stuff.wallet_address,
                signing_key,
                &meta_data,
            );

            match att.1 {
                Ok(y) => (
                    att.0,
                    Ok(Json(NewReservationResponse {
                        nft_id: uuid,
                        metadata_response: y,
                    })),
                ),
                Err(e) => (att.0, Err(e)),
            }
        }
        Err(e) => (result.0, Err(e)),
    }
}

#[get("/in-process")]
async fn get_in_process(
    conn: NFTDatabase,
) -> (Status, Result<Json<Vec<String>>, Json<ErrorResponse>>) {
    match conn.run(move |c| reservations_in_process(c, 100)).await {
        Ok(x) => (Status::new(200), Ok(Json(x))),
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}
#[get("/in-mint-process")]
async fn get_in_mint_process(
    conn: NFTDatabase,
) -> (
    Status,
    Result<Json<Vec<(String, String)>>, Json<ErrorResponse>>,
) {
    match conn
        .run(move |c| reservations_in_mint_process(c, 100))
        .await
    {
        Ok(x) => (Status::new(200), Ok(Json(x))),
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}
#[get("/stuck-mint-process")]
async fn get_stuck_mint_process(
    conn: NFTDatabase,
) -> (
    Status,
    Result<Json<Vec<MintReservation>>, Json<ErrorResponse>>,
) {
    match conn
        .run(move |c| reservations_stuck_in_mint_process(c, 100))
        .await
    {
        Ok(x) => (Status::new(200), Ok(Json(x))),
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}
/// These have been reserved by the system previously, but don't have a TX ID/in_process flag set
#[get("/in-mint-reserved")]
async fn get_in_mint_reserved(
    conn: NFTDatabase,
) -> (Status, Result<Json<Vec<String>>, Json<ErrorResponse>>) {
    match conn
        .run(move |c| reservations_in_mint_reserved(c, 100))
        .await
    {
        Ok(x) => (Status::new(200), Ok(Json(x))),
        Err(e) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: e.to_string(),
            })),
        ),
    }
}

#[get("/free/stage/<stage>")]
async fn get_free_stage(
    conn: NFTDatabase,
    signature: SignatureB64,
    stage: String,
    state: &State<ReservationState>,
) -> (
    Status,
    Result<Json<Vec<MintReservation>>, Json<ErrorResponse>>,
) {
    let ss = format!("{{\"stage\":\"{}\"}}", stage.to_string());

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
    conn.run(move |c| match get_stage(c, &stage) {
        Ok(stage_opt) => {
            if let Some(stage_rec) = stage_opt {
                if stage_rec.stage_free {
                    match get_open_wallets_for_stage(c, stage_rec.id) {
                        Ok(rows) => {
                            let mut reservations_generated: Vec<MintReservation> =
                                Default::default();
                            for row in rows {
                                let amount = row.allocated - row.assigned - row.reserved;
                                if amount > 0 {
                                    let reservations_result = mint_nft_for_wallet_in_stage(
                                        c,
                                        &stage_rec,
                                        &row.wallet_address,
                                        1,
                                    );
                                    match reservations_result {
                                        Err(e) => {
                                            return (e.0, Err(e.1));
                                        }
                                        Ok(reservations) => {
                                            reservations.iter().for_each(|f| {
                                                reservations_generated.push(f.clone())
                                            });
                                        }
                                    }
                                }
                            }
                            (Status::new(200), Ok(Json(reservations_generated)))
                        }
                        Err(e) => (
                            Status::new(500),
                            Err(Json(ErrorResponse {
                                code: 500,
                                message: e.to_string(),
                            })),
                        ),
                    }
                } else {
                    (
                        Status::new(403),
                        Err(Json(ErrorResponse {
                            code: 403,
                            message: "Stage is not free".to_string(),
                        })),
                    )
                }
            } else {
                (
                    Status::new(404),
                    Err(Json(ErrorResponse {
                        code: 404,
                        message: "stage not found".to_string(),
                    })),
                )
            }
        }
        Err(e) => (e.0, Err(e.1)),
    })
    .await
}

pub fn get_routes() -> Vec<Route> {
    routes![
        get_by_address,
        new_reservation,
        options_new_reservation,
        get_in_process,
        get_in_mint_process,
        get_in_mint_reserved,
        get_free_stage,
        get_stuck_mint_process
    ]
}
