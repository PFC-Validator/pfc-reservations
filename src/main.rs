pub mod auth;
pub mod catchers;
pub mod db;
pub mod handlers;
pub mod models;
pub mod requests;

#[macro_use]
extern crate rocket;

use chrono::Duration;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::figment::{
    util::map,
    value::{Map, Value},
};
use rocket::http::Header;
use rocket::{Build, Request, Response, Rocket};
use std::env;

use rocket_sync_db_pools::database;
use secp256k1::{All, Secp256k1};
use terra_rust_api::PrivateKey;
pub struct CORS {
    pub allowed_origins: String,
}

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "CORS Headers",
            kind: Kind::Response,
        }
    }
    async fn on_response<'r>(&self, _req: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new(
            "Access-Control-Allow-Origin",
            self.allowed_origins.clone(),
        ));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range,X-Reservation-Signature",
        ));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
#[database("NFT")]
pub struct NFTDatabase(rocket_sync_db_pools::postgres::Client);
pub struct ReservationState {
    pub signing_key: PrivateKey,
    pub verification_key: String,
    pub max_reservations: usize,
    pub max_reservation_duration: Duration,
    pub debug_mode: bool,
    pub chain: String,
    pub lcd: String,
    pub fcd: String,
    pub nft_contract: String,
}

#[launch]
fn rocket() -> Rocket<Build> {
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
    let allowed_url = env::var("ALLOWED_ORIGINS").expect("Missing CORS host(s)");
    let cors: CORS = CORS {
        allowed_origins: allowed_url,
    };

    let max_reservation_duration: i64 = env::var("MAX_RESERVATION_DURATION")
        .unwrap()
        .parse()
        .unwrap();
    let lcd = env::var("LCD_URL").expect("Missing LCD_URL Server in environment");
    let fcd = env::var("FCD_URL").expect("Missing FCD_URL server in environment");
    let chain = env::var("CHAIN_ID").expect("Missing CHAIN_ID server in environment");
    let nft_contract =
        env::var("NFT_CONTRACT").expect("Missing NFT_CONTRACT server in environment");
    let reservation_state = ReservationState {
        signing_key,
        verification_key: public_key,
        max_reservations,
        max_reservation_duration: Duration::minutes(max_reservation_duration),
        debug_mode,
        lcd,
        fcd,
        chain,
        nft_contract,
    };
    let db: Map<_, Value> = map! {"url"=>db_url.into(),"pool_size"=>pool_size.into()};
    let figment = rocket::Config::figment().merge(("databases", map!["NFT"=>db]));
    if debug_mode {
        log::error!("RUNNING IN DEBUG MODE: Signature generation/verification omitted")
    }
    rocket::custom(figment)
        .manage(reservation_state)
        .attach(NFTDatabase::fairing())
        .attach(cors)
        .register("/", catchers::get_catchers())
        .mount("/nft", handlers::nft::get_routes())
        .mount("/reservation", handlers::reservation::get_routes())
        .mount("/mint", handlers::mint::get_routes())
}
