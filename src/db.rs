use chrono::{DateTime, Utc};
use pfc_reservation::requests::{ErrorResponse, NFTTallyStat, NewReservationResponse, Reservation};
use postgres::{Client, Error, Statement};
use rocket::http::Status;
use rocket::serde::json::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::models::{Stage, WalletStageAllocation};
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
//fn clear_reservations(_conn: &mut Client) -> Result<i64, (Status, Json<ErrorResponse>)> {
//    todo!()
//}
/// examine available NFTs and 'reserve' one
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
    let available_stages = get_open_stages_for_wallet(conn, wallet_address);
    match available_stages {
        Ok(stages) => {
            if stages.is_empty() {
                Err((
                    Status::new(444),
                    Json(ErrorResponse {
                        code: 444,
                        message: "No stages are open for your wallet at this time".into(),
                    }),
                ))
            } else {
                // go through the available stages and try to allocate a NFT from each stage
                // get_open_stages should return the 'open' stage if it is open as a last resort
                let _r = conn.execute("select setseed($1)", &[&seed]);
                log::info!("_r = {:?}", _r);
                for stage in stages {
                    let query = if let Some(att_type) = stage.attribute_type {
                        if let Some(att_value) = stage.attribute_value {
                            log::info!("Stage: {} {}/{}", stage.code, att_type, att_value);
                            let select_stmt = r#"                                
                                update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2
                                where id = (
                                    select id as available
                                    from nft n, json_array_elements(n.meta_data -> 'attributes' ) att
                                    where assigned = false and (reserved=false or reserved_until < now())
                                     and  att ->> 'trait_type' = $3 and att ->> 'value' = $4
                                     and has_submit_error=false
                                    order by random()
                                    limit 1
                                ) returning id "#;
                            let stmt_reserve_nft: Statement = conn.prepare(select_stmt).unwrap();
                            conn.query_one(
                                &stmt_reserve_nft,
                                &[&String::from(wallet_address), pg_ts, &att_type, &att_value],
                            )
                        } else {
                            return Err((
                                Status::new(500),
                                Json(ErrorResponse {
                                    code: 500,
                                    message: format!("Stage: {} is misconfigured", stage.code),
                                }),
                            ));
                        }
                    } else {
                        log::info!("Stage: {} ", stage.code);
                        let select_stmt = r#"        
                            update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2
                            where id = (
                                select id as available
                                from nft
                                where assigned = false and (reserved=false or reserved_until < now())                   
                                and has_submit_error=false
                                order by random()
                                limit 1
                            ) returning id "#;
                        let stmt_reserve_nft: Statement = conn.prepare(select_stmt).unwrap();
                        conn.query_one(&stmt_reserve_nft, &[&String::from(wallet_address), pg_ts])
                    };

                    match query {
                        Ok(row) => {
                            let r = increase_stage_reservation(conn, stage.id, wallet_address);
                            if let Err(db_err) = r {
                                log::error!(
                                    "Wallet Whitelist has error {}. We still reserved",
                                    db_err.to_string()
                                )
                            }
                            let id_returned: Uuid = row.get(0);
                            return Ok(id_returned);
                        }
                        Err(db_err) => {
                            return Err((
                                Status::new(500),
                                Json(ErrorResponse {
                                    code: 500,
                                    message: db_err.to_string(),
                                }),
                            ))
                        }
                    }
                }
                Err((
                    Status::new(444),
                    Json(ErrorResponse {
                        code: 444,
                        message: "No NFTs available for reservation at this time".into(),
                    }),
                ))
            }
        }
        Err(e) => Err(e),
    }
}

pub fn get_stages(conn: &mut Client) -> Result<Vec<Stage>, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select id,code,name,attribute_type,attribute_value,is_default,stage_free,stage_open from stage_whitelist",
        &[],
    ) {
        Ok(row) => {

            let stages = row.iter().map (|r|{
                let code:String = r.get(1);
                Stage{
                id: r.get(0),
                code: code.trim().to_string(),
                name: r.get(2),
                attribute_type: r.get(3),
                attribute_value: r.get(4),
                is_default: r.get(5),
                stage_free: r.get(6),
                stage_open: r.get(7)
            }}).collect::<Vec<Stage>>();
           Ok( stages)
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

pub fn get_nft_stat(
    conn: &mut Client,
    attr_type: &Option<String>,
    attr_value: &Option<String>,
) -> Result<NFTTallyStat, (Status, Json<ErrorResponse>)> {
    let results = match attr_type {
        Some(a_t) => {
            if let Some(a_v) = attr_value {
                conn.query(
                    "Select sum(case assigned when true then 1  else 0 end),
       sum(case reserved when true then 1  else 0 end),
       sum(1) from nft n, json_array_elements(n.meta_data -> 'attributes' ) att
        where att ->> 'trait_type' = $1 and att ->> 'value' = $2",
                    &[&a_t, &a_v],
                )
            } else {
                return Err((
                    Status::new(500),
                    Json(ErrorResponse {
                        code: 500,
                        message: "Missing Value".into(),
                    }),
                ));
            }
        }
        None => conn.query(
            "Select sum(case assigned when true then 1 else 0 end),
       sum(case reserved when true then 1  else 0 end),
       sum(1) from nft",
            &[],
        ),
    };
    match results {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                Ok(NFTTallyStat {
                    assigned: row.get(0),
                    reserved: row.get(1),
                    count: row.get(2),
                })
            } else {
                Ok(NFTTallyStat {
                    assigned: 0,
                    reserved: 0,
                    count: 0,
                })
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
pub fn get_open_stage(
    mut conn: Client,
) -> Result<DateTime<chrono::offset::Utc>, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select min(stage_open) from stage_whitelist where is_default=true ",
        &[],
    ) {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                let date: DateTime<chrono::offset::Utc> = row.get(0);
                Ok(date)
            } else {
                Err((
                    Status::new(404),
                    Json(ErrorResponse {
                        code: 404,
                        message: String::from("No open stages are available"),
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
/*
pub(crate) fn validate_stage_for_wallet(
    mut conn: Client,
    wallet: &str,
    stage: &str,
) -> Result<WalletStageAllocation, (Status, Json<ErrorResponse>)> {
    let query = conn.query(
        "select w.id, allocation_count,reserved_count,assigned_count, s.stage_open
from wallet_whitelist w, stage_whitelist s
where s.code=$1
  and w.wallet_address=$2
and s.id = w.stage",
        &[&String::from(stage), &String::from(wallet)],
    );
    match query {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                Ok(WalletStageAllocation {
                    id: Some(row.get(0)),
                    allocation_count: row.get(1),
                    reserved_count: row.get(2),
                    assigned_count: row.get(3),
                    stage_open: Some(row.get(4)),
                })
            } else {
                Ok(WalletStageAllocation {
                    id: None,
                    allocation_count: 0,
                    reserved_count: 0,
                    assigned_count: 0,
                    stage_open: None,
                })
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

 */
pub fn get_open_stages_for_wallet(
    conn: &mut Client,
    wallet: &str,
) -> Result<Vec<Stage>, (Status, Json<ErrorResponse>)> {
    let query = conn.query(
        "select id,code,name,attribute_type,attribute_value,is_Default,stage_free,stage_open, 1 as sort_pref 
        from stage_whitelist where
        stage_open < now() and  
        id in (
    select stage
    from wallet_whitelist
    where wallet_address = $1
      and allocation_count > (reserved_count + wallet_whitelist.assigned_count)
)
union
select id,code,name,attribute_type,attribute_value,is_Default,stage_free,stage_open,2
from stage_whitelist
where
        stage_open < now() and
      is_default = true
order by sort_pref
",
        &[&String::from(wallet)],
    );
    match query {
        Ok(rows) => Ok(rows
            .iter()
            .map(|r| Stage {
                id: r.get(0),
                code: r.get(1),
                name: r.get(2),
                attribute_type: r.get(3),
                attribute_value: r.get(4),
                is_default: r.get(5),
                stage_free: r.get(6),
                stage_open: r.get(7),
            })
            .collect::<Vec<Stage>>()),
        Err(db_err) => Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        )),
    }
}
pub fn increase_stage_reservation(
    conn: &mut Client,
    stage_id: Uuid,
    wallet_address: &str,
) -> Result<u64, Error> {
    conn.execute(
        "update wallet_whitelist set reserved_count =reserved_count+ 1 where wallet_address=$1 and stage = $2",
        &[&String::from(wallet_address), &stage_id],
    )
}
