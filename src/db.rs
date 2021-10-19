use chrono::{DateTime, Utc};
use num_traits::cast::ToPrimitive;
use pfc_reservation::requests::{ErrorResponse, NewReservationResponse, Reservation};
use postgres::{Client, Statement};
use rocket::http::Status;
use rocket::serde::json::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use uuid::Uuid;
// examine available NFTs and 'reserve' one
pub fn get_reservation_count(
    conn: &mut Client,
    wallet_address: &str,
) -> Result<usize, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select count(*) from NFT where (reserved_to_wallet_address=$1 and reserved=true and reserved_until > now()) or assigned_to_wallet_address=$2",
        &[&String::from(wallet_address),&String::from(wallet_address)],
    ) {
        Ok(reservation_count) => {
            if let Some(row) = reservation_count.first() {
                let id_returned: i64 = row.get(0);

                Ok(id_returned.unsigned_abs() as usize)
            } else {
                Err((
                    Status::new(500),
                    Json(ErrorResponse {
                        code: 444,
                        message: "No NFTs available for reservation at this time".into(),
                    }),
                ))
            }
        }
        Err(db_err) => Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        )),
    }
}

// examine available NFTs and 'reserve' one
pub fn get_reservations_for_wallet(
    conn: &mut Client,
    wallet_address: &str,
) -> (Status, Result<Json<Vec<Reservation>>, Json<ErrorResponse>>) {
    match conn.query(
        r#"
        Select  reserved_to_wallet_address, id, reserved_until, reserved,  assigned,  assigned_on, has_submit_error
        from NFT
        where (reserved_to_wallet_address=$1 and reserved=true and reserved_until > now()) or assigned_to_wallet_address=$2"#,
        &[&String::from(wallet_address),&String::from(wallet_address)],
    ) {
        Ok(reservation_rows) => {
            let reservations = reservation_rows.iter().map(|r| {
                let mut reserved:bool = r.get(3);
                let mut reserved_until :Option<DateTime<chrono::offset::Utc>>= r.get(2);
                let wallet_return:String = r.get(0);
                if wallet_address != wallet_return {
                    reserved=false;
                    reserved_until= None
                }
                Reservation{
                wallet_address: wallet_address.to_string(),
                nft_id: r.get(1),
                reserved,
                reserved_until,
                assigned: r.get(4),
                assigned_on: r.get(5),
                has_submit_error: r.get(6)
            }}).collect::<Vec<Reservation>>();

                (Status::new(200), Ok(Json(reservations)))

        }
        Err(db_err) => (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        )),
    }
}
/// do a reservation for a NFT, picking NFT in seemingly random order
pub fn do_reservation(
    mut c: &mut Client,
    wallet_address: &str,
    reserved_until: &DateTime<Utc>,
    max_reservations: usize,
) -> (
    Status,
    Result<Json<NewReservationResponse>, Json<ErrorResponse>>,
) {
    let res_count_r = get_reservation_count(c, wallet_address);
    match res_count_r {
        Ok(count) => {
            if count >= max_reservations {
                (
                    Status::new(403),
                    Err(Json(ErrorResponse {
                        code: 403,
                        message: "Reservation limit exceeded".to_string(),
                    })),
                )
            } else {
                let nft_id_r =
                    get_and_reserve_available_nft(&mut c, wallet_address, reserved_until);
                match nft_id_r {
                    Ok(nft_id) => (
                        Status::new(201),
                        Ok(Json(NewReservationResponse { nft_id })),
                    ),

                    Err(e) => (e.0, Err(e.1)),
                }
            }
        }
        Err(err) => (err.0, Err(err.1)),
    }
}

/// clear reservations from expired reservations
fn clear_reservations(_conn: &mut Client) -> Result<i64, (Status, Json<ErrorResponse>)> {
    todo!()
}
// examine available NFTs and 'reserve' one
pub fn get_and_reserve_available_nft(
    conn: &mut Client,
    wallet_address: &str,
    reserved_until: &DateTime<Utc>,
) -> Result<Uuid, (Status, Json<ErrorResponse>)> {
    let pg_ts: &DateTime<chrono::offset::Utc> = reserved_until;
    let mut hasher = DefaultHasher::new();
    wallet_address.hash(&mut hasher);
    let hash = hasher.finish();
    let hash_i32: i32 = (((hash % i32::MAX as u64) as i32) - (i32::MAX / 2)) as i32;
    let hash_f64: f64 = f64::from(hash_i32);

    let seed: f64 = if hash_f64 == 0.0 {
        -1.0
    } else {
        hash_f64 / f64::from(i32::MAX)
    };
    log::info!("Seed for {} is {} {}", wallet_address, hash, seed);

    let stmt_reserve_nft: Statement = conn
        .prepare(
            r#"
          
                update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2
                where id = (
                    select id as available
                    from nft
                    where assigned = false and (reserved=false or reserved_until < now())
                    and  setseed($3) is not null
                    order by random()
                    limit 1
                ) returning id "#,
        )
        .unwrap();

    match conn.query(
        &stmt_reserve_nft,
        &[&String::from(wallet_address), pg_ts, &seed],
    ) {
        Ok(reserved_nft) => {
            if let Some(row) = reserved_nft.first() {
                let id_returned: Uuid = row.get(0);
                Ok(id_returned)
            } else {
                Err((
                    Status::new(500),
                    Json(ErrorResponse {
                        code: 444,
                        message: "No NFTs available for reservation at this time".into(),
                    }),
                ))
            }
        }
        Err(db_err) => Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        )),
    }
}
