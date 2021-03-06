use crate::requests::{ErrorResponse, NFTTallyStat, Reservation};
use chrono::{DateTime, Utc};
use postgres::{Client, Error, Row, Statement};
use rocket::http::Status;
use rocket::serde::json::Json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Add;

use crate::models::{NftFull, Stage, NFT};
use crate::requests::Metadata;
use crate::requests::{MintReservation, OpenStageWallet};
use uuid::Uuid;

// examine available NFTs and 'reserve' one
pub fn get_reservation_count(
    conn: &mut Client,
    wallet_address: &str,
) -> Result<usize, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select count(*) from NFT where (reserved_to_wallet_address=$1 and reserved=true and (in_process or reserved_until > now())) or assigned_to_wallet_address=$2",
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
        Err(db_err) => {
            log::error!("get_reservation_count: {}", db_err.to_string());
            Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        ))},
    }
}

// examine available NFTs and 'reserve' one
pub fn get_reservations_for_wallet(
    conn: &mut Client,
    wallet_address: &str,
) -> (Status, Result<Json<Vec<Reservation>>, Json<ErrorResponse>>) {
    match conn.query(
        r#"
        Select  reserved_to_wallet_address, id, reserved_until, reserved,  assigned,  assigned_on, has_submit_error, in_process, txhash, tx_error, tx_retry_count,token_id
        from NFT
        where (reserved_to_wallet_address=$1 and ((reserved=true and reserved_until > now()) or in_process=true) ) or assigned_to_wallet_address=$2"#,
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
                let txhash:Option<String> = r.get(8);
                let tx_error:Option<String> = r.get(9);
                let assigned:bool = r.get(4);
                let token_id:Option<String> = if assigned {
                    r.get(11)
                } else {
                    None
                };
                Reservation {
                    wallet_address: wallet_address.to_string(),
                    nft_id: r.get(1),
                    reserved,
                    reserved_until,
                    assigned,
                    assigned_on: r.get(5),
                    has_submit_error: r.get(6),
                    in_process: r.get(7),
                    tx_hash: txhash,
                    tx_error,
                    tx_retry_count: r.get(10),
                    token_id
                }
            }).collect::<Vec<Reservation>>();

                (Status::new(200), Ok(Json(reservations)))

        }
        Err(db_err) => {
            log::error!("get_reservations_for_wallet: {}", db_err.to_string());
            (
            Status::new(500),
            Err(Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        ))},
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
    Result<(Uuid, serde_json::Value), Json<ErrorResponse>>,
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
                    Ok(nft_reservation) => {
                        let nft_id = nft_reservation.0;
                        let meta = nft_reservation.1;
                        (Status::new(201), Ok((nft_id, meta)))
                    }

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
) -> Result<(Uuid, serde_json::Value), (Status, Json<ErrorResponse>)> {
    //  let pg_ts: &DateTime<chrono::offset::Utc> = reserved_until;
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
                    let query = do_reservation_in_stage(
                        conn,
                        &stage,
                        wallet_address,
                        1,
                        false,
                        reserved_until,
                    );
                    match query {
                        Ok(rows) => {
                            log::info!("get_and_reserve_available_nft/rows={}", rows.len());
                            if let Some(row) = rows.first() {
                                let r =
                                    increase_stage_reservation(conn, stage.id, wallet_address, 1);
                                if let Err(db_err) = r {
                                    log::error!(
                                        "Wallet Whitelist has error {}. We still reserved",
                                        db_err.to_string()
                                    )
                                }
                                let id_returned: Uuid = row.get(0);
                                let meta_data: serde_json::Value = row.get(1);
                                return Ok((id_returned, meta_data));
                            } else {
                                log::info!(
                                    "Stage {}-{} full.. off to next one",
                                    stage.code,
                                    stage.name
                                )
                            }
                        }
                        Err(db_err) => {
                            return {
                                log::error!("get_and_reserve_available_nft: {}", db_err.1.message);
                                Err(db_err)
                            }
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
/// get a single stage
pub fn get_stage(
    conn: &mut Client,
    code: &str,
) -> Result<Option<Stage>, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select id,code,name,attribute_type,attribute_value,is_default,stage_free,stage_open,stage_close from stage_whitelist where code=$1",
        &[&String::from(code)],
    ) {
        Ok(rows) => {

            let stage = rows.first().map (|r|{
                let code:String = r.get(1);
                Stage{
                    id: r.get(0),
                    code: code.trim().to_string(),
                    name: r.get(2),
                    attribute_type: r.get(3),
                    attribute_value: r.get(4),
                    is_default: r.get(5),
                    stage_free: r.get(6),
                    stage_open: r.get(7),
                    stage_close: r.get(8)
                }});
            Ok(stage)
        }
        Err(db_err) => {
            log::error!("get_stage: {}", db_err.to_string());
            Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        ))},
    }
}
/// get a collection of stages
pub fn get_stages(conn: &mut Client) -> Result<Vec<Stage>, (Status, Json<ErrorResponse>)> {
    match conn.query(
        "Select id,code,name,attribute_type,attribute_value,is_default,stage_free,stage_open, stage_close from stage_whitelist",
        &[],
    ) {
        Ok(rows) => {

            let stages = rows.iter().map (|r|{
                let code:String = r.get(1);
                Stage{
                id: r.get(0),
                code: code.trim().to_string(),
                name: r.get(2),
                attribute_type: r.get(3),
                attribute_value: r.get(4),
                is_default: r.get(5),
                stage_free: r.get(6),
                stage_open: r.get(7),
                stage_close : r.get(8)
                }
            }).collect::<Vec<Stage>>();
           Ok( stages)
        }
        Err(db_err) =>{
            log::error!("get_stages: {}", db_err.to_string());
            Err((
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: db_err.to_string(),
            }),
        ))},
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
       sum(case reserved when true then 1 else 0 end),
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
        Err(db_err) => {
            log::error!("get_nft_stat: {}", db_err.to_string());
            Err((
                Status::new(500),
                Json(ErrorResponse {
                    code: 500,
                    message: db_err.to_string(),
                }),
            ))
        }
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
        Err(db_err) => {
            log::error!("get_open_stage: {}", db_err.to_string());
            Err((
                Status::new(500),
                Json(ErrorResponse {
                    code: 500,
                    message: db_err.to_string(),
                }),
            ))
        }
    }
}

/// for regular reservations, get a list of 'special stages/whitelists' that the wallet is entitled too
pub fn get_open_stages_for_wallet(
    conn: &mut Client,
    wallet: &str,
) -> Result<Vec<Stage>, (Status, Json<ErrorResponse>)> {
    let query = conn.query(
        "select id,code,name,attribute_type,attribute_value,is_Default,stage_free,stage_open, stage_close, 1 as sort_pref 
        from stage_whitelist where
        stage_open < now() and  
        id in (
    select stage
    from wallet_whitelist
    where wallet_address = $1
      and allocation_count > (reserved_count + wallet_whitelist.assigned_count)
)
union
select id,code,name,attribute_type,attribute_value,is_Default,stage_free,stage_open,stage_close,2
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
                stage_close: r.get(8),
            })
            .collect::<Vec<Stage>>()),
        Err(db_err) => {
            log::error!("get_open_stages_for_wallet:{}", db_err.to_string());
            Err((
                Status::new(500),
                Json(ErrorResponse {
                    code: 500,
                    message: db_err.to_string(),
                }),
            ))
        }
    }
}
/// update wallet reservation count
pub fn increase_stage_reservation(
    conn: &mut Client,
    stage_id: Uuid,
    wallet_address: &str,
    amount: i32,
) -> Result<u64, Error> {
    conn.execute(
        "update wallet_whitelist set reserved_count =reserved_count+ $3 where wallet_address=$1 and stage = $2",
        &[&String::from(wallet_address), &stage_id,&amount],
    )
}
/// retried NFT from database
pub fn get_nft(conn: &mut Client, nft: &Uuid) -> Result<NftFull, Error> {
    conn.query_one(
        r#"
            Select  id,name, assigned, reserved, has_submit_error, reserved_until, 
                    meta_data, svg, ipfs_image, ipfs_meta, image_data, external_url, description, background_color, 
                    animation_url, youtube_url, assigned_on, assigned_to_wallet_address, reserved_to_wallet_address,signed_packet ,in_process,txhash
                    from NFT where id = $1"#,
        &[nft],
    )
    .map(|r| {
        let n = NFT{
            id: r.get(0),
            name: r.get(1),
            assigned: r.get(2),
            reserved: r.get(3),
            has_submit_error: r.get(4),
            reserved_until: r.get(5),
            in_process:r.get(20),
            txhash:r.get(21)
        };
        NftFull {
            nft_lite: n,
            meta_data: r.get(6),
            svg: r.get(7),
            ipfs_image: r.get(8),
            ipfs_meta: r.get(9),
            image_data:r.get(10),
            external_url: r.get(11),
            description: r.get(12),
            background_color: r.get(13),
            animation_url: r.get(14),
            youtube_url: r.get(15),
            assigned_on: r.get(16),
            assigned_to_wallet_address: r.get(17),
            reserved_to_wallet_address: r.get(18),
            signed_packet: r.get(19)
        }})
}

/// set TXHash for NFT purchase, and set NFT 'in_progress'
pub fn set_tx_hash_for_nft(conn: &mut Client, nft: &Uuid, txhash: &str) -> Result<u64, Error> {
    conn.execute(
        "update NFT set txhash = $1, in_process = true, has_submit_error =false, tx_retry_count = tx_retry_count+1 where id = $2",
        &[&String::from(txhash), &nft],
    )
}
/// set TX for NFT purchase, and set NFT 'in_progress'
pub fn set_tx_for_nft(conn: &mut Client, nft: &Uuid, tx: &str) -> Result<u64, Error> {
    conn.execute(
        "update NFT set signed_packet = $1, in_process = true where id = $2",
        &[&String::from(tx), &nft],
    )
}

pub fn is_name_available(conn: &mut Client, name: &str) -> Result<bool, Error> {
    let query = conn.query_one(
        "select count(*) from NFT where upper(name) = upper($1) or upper(token_id) = upper($1)",
        &[&String::from(name)],
    );
    match query {
        Ok(row) => {
            let cnt: i64 = row.get(0);
            Ok(cnt == 0)
        }
        Err(db_err) => Err(db_err),
    }
}

pub fn reservations_in_process(conn: &mut Client, limit: i64) -> Result<Vec<String>, Error> {
    let query = conn.query(
        "select txhash from nft where in_process = true and has_submit_error=false and in_mint_run=false limit $1",
        &[&limit],
    );
    match query {
        Ok(rows) => Ok(rows.iter().map(|r| r.get(0)).collect::<Vec<String>>()),
        Err(db_err) => Err(db_err),
    }
}

pub fn reservations_in_mint_process(
    conn: &mut Client,
    limit: i64,
) -> Result<Vec<(String, String)>, Error> {
    let query = conn.query(
        "select txhash, name from nft where in_process = true and has_submit_error=false and in_mint_run=true limit $1",
        &[&limit],
    );
    match query {
        Ok(rows) => Ok(rows
            .iter()
            .map(|r| (r.get(0), r.get(1)))
            .collect::<Vec<(String, String)>>()),
        Err(db_err) => Err(db_err),
    }
}
pub fn reservations_stuck_in_mint_process(
    conn: &mut Client,
    limit: i64,
) -> Result<Vec<MintReservation>, Error> {
    let query = conn.query(
        "select reserved_to_wallet_address, id,meta_data from nft where in_process = false and assigned=false and has_submit_error=false and in_mint_run=true limit $1",
        &[&limit],
    );
    match query {
        Ok(rows) => Ok(rows
            .iter()
            .map(|r| {
                let meta: Metadata = serde_json::from_value(r.get(2)).unwrap();
                MintReservation {
                    wallet_address: r.get(0),
                    nft_id: r.get(1),
                    meta_data: meta,
                }
            })
            .collect::<Vec<MintReservation>>()),
        Err(db_err) => Err(db_err),
    }
}
pub fn reservations_in_mint_reserved(conn: &mut Client, limit: i64) -> Result<Vec<String>, Error> {
    let query = conn.query(
        "select name from nft where assigned=false and reserved = true and has_submit_error=false and in_mint_run=true limit $1",
        &[&limit],
    );
    match query {
        Ok(rows) => Ok(rows.iter().map(|r| r.get(0)).collect::<Vec<String>>()),
        Err(db_err) => Err(db_err),
    }
}

pub fn nft_assign_tx_result(
    conn: &mut Client,
    wallet: Option<String>,
    txhash: String,
    result: bool,
    tx_time: Option<DateTime<chrono::offset::Utc>>,
    error_message: Option<String>,
    token_id: Option<String>,
) -> Result<u64, Error> {
    if result {
        conn.execute(
            "update nft set has_submit_error=false, in_process=false, assigned=true, reserved=false, tx_error=null, assigned_to_wallet_address=$1, assigned_on=$2, token_id=$3 where txhash=$4",
            &[&wallet,&tx_time, &token_id, &txhash],
        )
    } else {
        conn.execute(
            "update nft set has_submit_error=true, tx_error=$1 where txhash=$2",
            &[&error_message, &txhash],
        )
    }
}

pub fn nft_assign_owner(conn: &mut Client, wallet: String, token_id: String) -> Result<u64, Error> {
    log::debug!("nft_assign_owner: {} {}", wallet, token_id);
    conn.execute(
            r#"update nft set has_submit_error=false, in_process=false, assigned=true, reserved=false, tx_error=null, assigned_to_wallet_address=$1, token_id=$2 
            where reserved_to_wallet_address=$3 and name=$4"#,
            &[&wallet, &token_id,&wallet,&token_id],
        )
}

/// get a list of open wallets for a stage
pub fn get_open_wallets_for_stage(
    conn: &mut Client,
    stage_id: Uuid,
) -> Result<Vec<OpenStageWallet>, Error> {
    let query = conn.query(
        r#"select wallet_address, allocation_count, reserved_count, assigned_count
            from wallet_whitelist
             where stage = $1  
            and allocation_count > ( reserved_count + assigned_count) "#,
        &[&stage_id],
    );
    match query {
        Ok(rows) => Ok(rows
            .iter()
            .map(|r| OpenStageWallet {
                wallet_address: r.get(0),
                stage_id,
                allocated: r.get(1),
                reserved: r.get(2),
                assigned: r.get(3),
            })
            .collect::<Vec<OpenStageWallet>>()),
        Err(db_err) => {
            log::error!("get_open_wallets_for_stage:{}", db_err);
            Err(db_err)
        }
    }
}
pub fn mint_nft_for_wallet_in_stage(
    conn: &mut Client,
    stage: &Stage,
    wallet_address: &str,
    amount: i64,
) -> Result<Vec<MintReservation>, (Status, Json<ErrorResponse>)> {
    let close = stage
        .stage_close
        .unwrap_or_else(|| chrono::Utc::now().add(chrono::Duration::hours(24)));
    let query = do_reservation_in_stage(conn, stage, wallet_address, amount, true, &close);
    match query {
        Ok(rows) => {
            log::debug!("mint_nft_for_wallet_in_stage/rows={}", rows.len());
            let _r = increase_stage_reservation(conn, stage.id, wallet_address, rows.len() as i32)
                .map_err(|e| {
                    log::error!("mint_nft_for_wallet_in_stage:{}", e.to_string());
                    (
                        Status::InternalServerError,
                        Json(ErrorResponse {
                            code: 500,
                            message: e.to_string(),
                        }),
                    )
                })?;

            Ok(rows
                .iter()
                .map(|row| {
                    let meta: Metadata = serde_json::from_value(row.get(1)).unwrap();
                    MintReservation {
                        wallet_address: wallet_address.to_string(),
                        nft_id: row.get(0),
                        meta_data: meta,
                    }
                })
                .collect::<Vec<MintReservation>>())
        }
        Err(e) => {
            log::error!("mint_nft_for_wallet_in_stage: {}", e.1.message);
            Err(e)
        }
    }
}
pub fn do_reservation_in_stage(
    conn: &mut Client,
    stage: &Stage,
    wallet_address: &str,
    amount: i64,
    is_mint: bool,
    reserved_until: &DateTime<Utc>,
) -> Result<Vec<Row>, (Status, Json<ErrorResponse>)> {
    let pg_ts: &DateTime<chrono::offset::Utc> = reserved_until;
    let query = if &stage.code == "bagel" {
        let select_stmt = r#"                                
                                update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2, in_mint_run=$4
                                where id in (
                                    select id as available
                                    from nft n, json_array_elements(n.meta_data -> 'attributes' ) att
                                    where assigned = false 
                                     and (reserved=false or reserved_until < now())
                                     and name =$3
                                     and has_submit_error=false
                                     and in_process=false
                                    order by random()
                                    limit 1
                                ) returning id,meta_data "#;
        let stmt_reserve_nft: Statement = conn.prepare(select_stmt).unwrap();
        conn.query(
            &stmt_reserve_nft,
            &[
                &String::from(wallet_address),
                pg_ts,
                &String::from("Evan Bagelmeister"),
                &is_mint,
            ],
        )
    } else if let Some(att_type) = &stage.attribute_type {
        if let Some(att_value) = &stage.attribute_value {
            log::info!(
                "Stage: {} {}/{} - {}",
                stage.code,
                att_type,
                att_value,
                wallet_address
            );
            let select_stmt = r#"                                
                                update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2, in_mint_run=$6
                                where id in (
                                    select id as available
                                    from nft n, json_array_elements(n.meta_data -> 'attributes' ) att
                                    where assigned = false 
                                     and (reserved=false or reserved_until < now())
                                     and  att ->> 'trait_type' = $3 and att ->> 'value' = $4
                                     and has_submit_error=false
                                     and in_process=false
                                    order by random()
                                    limit $5
                                ) returning id,meta_data "#;
            let stmt_reserve_nft: Statement = conn.prepare(select_stmt).unwrap();
            conn.query(
                &stmt_reserve_nft,
                &[
                    &String::from(wallet_address),
                    pg_ts,
                    &att_type,
                    &att_value,
                    &amount,
                    &is_mint,
                ],
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
        log::info!("Stage: {} - {}", stage.code, wallet_address);
        let select_stmt = r#"        
                            update nft set reserved=true, reserved_to_wallet_address=$1 ,reserved_until=$2, in_mint_run=$4
                            where id in (
                                select id as available
                                from nft
                                where assigned = false 
                                and (reserved=false or reserved_until < now())                   
                                and has_submit_error=false
                                and in_process=false
                                order by random()
                                limit $3
                            ) returning id,meta_data "#;
        let stmt_reserve_nft: Statement = conn.prepare(select_stmt).unwrap();
        conn.query(
            &stmt_reserve_nft,
            &[&String::from(wallet_address), pg_ts, &amount, &is_mint],
        )
    };
    query.map_err(|db_err| {
        log::error!("do_reservation_in_stage: {}", db_err.to_string());
        (
            Status::new(500),
            Json(ErrorResponse {
                code: 500,
                message: format!("{}", db_err),
            }),
        )
    })
}
