use pfc_reservation::requests::NewNFTRequest;
use secp256k1::{All, Secp256k1};
use std::env;
use terra_rust_api::PrivateKey;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("This program generates a signature for testing 'new' NFT generation");
        eprintln!("Usage: gen-new <name> <meta> <svg>");
    } else {
        let new_nft = NewNFTRequest {
            name: args.get(1).unwrap().clone(),
            meta: args.get(2).unwrap().clone(),
            svg: args.get(3).unwrap().clone(),
            ipfs_image: "".to_string(),
            ipfs_meta: "".to_string(),
            image_data: None,
            external_url: None,
            description: None,
            background_color: None,
            animation_url: None,
            youtube_url: None
        };
        let json = serde_json::to_string(&new_nft).unwrap();
        let secp: Secp256k1<All> = Secp256k1::new();
        let signing_key_phrase = env::var("DEBUG_RESERVATION_AUTH")
            .expect("Environment Variable 'RESERVATION_AUTH' Not present");

        let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();
        let sig = signing_key.sign(&secp, &json).unwrap();
        println!("Message:\n{}", json);
        println!("Signature:\n{}", sig.signature);
        println!("Public Key:\n{}", sig.pub_key.value);
    }
}
