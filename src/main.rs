pub mod auth;
pub mod catchers;
pub mod db;
pub mod handlers;
pub mod models;
pub mod requests;

#[macro_use]
extern crate rocket;

use chrono::Duration;
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use std::env;
//use diesel::pg::PgConnection;
//use diesel::prelude::*;
use rocket_sync_db_pools::database;
use secp256k1::{All, Secp256k1};
use terra_rust_api::PrivateKey;
//use rocket_sync_db_pools::diesel::prelude::*;

#[database("NFT")]
pub struct NFTDatabase(rocket_sync_db_pools::postgres::Client);
pub struct ReservationState {
    pub signing_key: PrivateKey,
    pub verification_key: String,
    pub max_reservations: usize,
    pub max_reservation_duration: Duration,
    pub debug_mode: bool,
}

#[launch]
fn rocket() -> _ {
    dotenv::dotenv().ok();
    env_logger::init();
    let secp: Secp256k1<All> = Secp256k1::new();
    let db_url = env::var("DATABASE_URL").unwrap();
    let pool_size = env::var("DATABASE_POOL")
        .unwrap_or_else(|_| "10".into())
        .parse::<usize>()
        .unwrap();
    let signing_key_phrase = env::var("RESERVATION_RESPONSE").unwrap();
    let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();
    let public_key = env::var("RESERVATION_AUTH_PUBLIC_KEY").unwrap();
    let max_reservations: usize = env::var("MAX_RESERVATIONS").unwrap().parse().unwrap();
    let debug_mode = match std::env::var("DEBUG_IGNORE_SIG") {
        Ok(x) => x == "true",
        Err(_) => false,
    };

    let max_reservation_duration: i64 = env::var("MAX_RESERVATION_DURATION")
        .unwrap()
        .parse()
        .unwrap();
    let reservation_state = ReservationState {
        signing_key,
        verification_key: public_key,
        max_reservations,
        max_reservation_duration: Duration::minutes(max_reservation_duration),
        debug_mode,
    };
    let db: Map<_, Value> = map! {"url"=>db_url.into(),"pool_size"=>pool_size.into()};
    let figment = rocket::Config::figment().merge(("databases", map!["NFT"=>db]));
    if debug_mode {
        log::error!("RUNNING IN DEBUG MODE: Signature generation/verification omitted")
    }
    rocket::custom(figment)
        .manage(reservation_state)
        .attach(NFTDatabase::fairing())
        .register("/", catchers::get_catchers())
        .mount("/nft", handlers::nft::get_routes())
        .mount("/reservation", handlers::reservation::get_routes())
        .mount("/mint", handlers::mint::get_routes())
}
