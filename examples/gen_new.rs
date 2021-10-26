use pfc_reservation::requests::NewNFTRequest;
use secp256k1::{All, Secp256k1};
use std::env;
use terra_rust_api::PrivateKey;
use terra_rust_wallet::Wallet;

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
            youtube_url: None,
        };
        //   let json = serde_json::to_string(&new_nft).unwrap();
        let secp: Secp256k1<All> = Secp256k1::new();
        let signing_key_phrase = env::var("DEBUG_RESERVATION_AUTH")
            .expect("Environment Variable 'DEBUG_RESERVATION_AUTH' Not present");

        let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();
        let json = r#"random/{"token_uri":"https://www.merriam-webster.com/dictionary/petrify","image":null,"image_data":null,"external_url":null,"description":null,"name":null,"attributes":[{"display_type":null,"trait_type":"gender","value":"male"},{"display_type":null,"trait_type":"name","value":"Jim Morrisson"}],"background_color":null,"animation_url":null,"youtube_url":null}"#;
        let sig = signing_key.sign(&secp, &json).unwrap();
        println!("Message:\n{}", json);
        println!("Signature:\n{}", sig.signature);
        println!("Public Key:\n{}", sig.pub_key.value);
    }
}
