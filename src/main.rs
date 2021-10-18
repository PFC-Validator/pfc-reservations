pub mod auth;
pub mod handlers;
pub mod models;
pub mod requests;

#[macro_use]
extern crate rocket;

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
}

#[launch]
fn rocket() -> _ {
    dotenv::dotenv().ok();
    env_logger::init();
    let secp: Secp256k1<All> = Secp256k1::new();
    let db_url = env::var("DATABASE_URL").unwrap();
    let pool_size = env::var("DATABASE_POOL")
        .unwrap_or("10".into())
        .parse::<usize>()
        .unwrap();
    let signing_key_phrase = env::var("RESERVATION_RESPONSE").unwrap();
    let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();
    let public_key = env::var("RESERVATION_AUTH_PUBLIC_KEY").unwrap();
    let reservation_state = ReservationState {
        signing_key,
        verification_key: public_key,
    };
    let db: Map<_, Value> = map! {"url"=>db_url.into(),"pool_size"=>pool_size.into()};
    let figment = rocket::Config::figment().merge(("databases", map!["NFT"=>db]));
    rocket::custom(figment)
        .manage(reservation_state)
        .attach(NFTDatabase::fairing())
        .mount("/nft", handlers::nft::get_routes())
}
