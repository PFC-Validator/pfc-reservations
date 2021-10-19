use chrono::{Duration, Utc};
use pfc_reservation::requests::NewReservationRequest;
use secp256k1::{All, Secp256k1};
use std::env;
use terra_rust_api::PrivateKey;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("This program generates a signature for testing 'new' NFT reservation");
        eprintln!("Usage: gen-index <wallet-name> <key-name> <duration-in-minutes>");
    } else {
        let secp: Secp256k1<All> = Secp256k1::new();
        let wallet_name = args.get(1).unwrap().clone();
        let key_name = args.get(2).unwrap().clone();
        let duration: i64 = args
            .get(3)
            .unwrap()
            .parse()
            .expect("Unable to parse duration number");
        let until = Utc::now() + Duration::minutes(duration);

        let wallet = terra_rust_wallet::Wallet::create(&wallet_name);

        let public_key = wallet
            .get_public_key(&secp, &key_name, None)
            .expect("Couldn't find key?");
        let wallet_address = public_key
            .account()
            .expect("Unable to obtain wallet address");
        let reservation_request = NewReservationRequest {
            wallet_address,
            reserved_until: until,
            signed_tx: None,
        };
        let json =
            serde_json::to_string(&reservation_request).expect("Unable to serialize request");

        let signing_key_phrase = env::var("DEBUG_RESERVATION_AUTH")
            .expect("Environment Variable 'RESERVATION_AUTH' Not present");

        let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();
        let sig = signing_key.sign(&secp, &json).unwrap();

        println!("Message:\n{}", &json);
        println!("Signature:\n{}", sig.signature);
        println!("Public Key:\n{}", sig.pub_key.value);
    }
}
