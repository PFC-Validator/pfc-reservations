use crate::auth::{is_valid_address, verify_signature, SignatureB64};
use crate::db::{do_reservation, get_reservations_for_wallet};
use crate::{NFTDatabase, ReservationState};
use chrono::Utc;
use pfc_reservation::requests::{
    ErrorResponse, NewReservationRequest, NewReservationResponse, Reservation,
};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State};

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
        if let Ok(debug) = std::env::var("DEBUG_IGNORE_SIG") {
            if debug == "true" {
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
    let x = conn
        .run(move |c| {
            do_reservation(
                c,
                &reservation_in_stuff.wallet_address,
                &reservation_in_stuff.reserved_until,
                max_reservations,
            )
        })
        .await;
    x
}

pub fn get_routes() -> Vec<Route> {
    routes![get_by_address, new_reservation]
}
