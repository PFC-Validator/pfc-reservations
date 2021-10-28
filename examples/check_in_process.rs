use anyhow::Error;
use dotenv::dotenv;
use pfc_reservation::requests::ReservationTxResultRequest;
use reqwest::Client;
use secp256k1::{All, Secp256k1};
use std::env;
use terra_rust_api::client::tx_types::TXResultBlock;
use terra_rust_api::{PrivateKey, Terra};

async fn update_tx(
    client: &Client,
    secp: &Secp256k1<All>,
    signing_key: &PrivateKey,
    server_url: &str,
    status: ReservationTxResultRequest,
) -> Result<(), Error> {
    let message = serde_json::to_string(&status)?;
    let url = format!("{}/mint/tx_result", server_url);
    log::info!("{}", url);
    let signature = signing_key.sign(secp, &message)?;
    log::info!("{}\t{}", signature.signature, signature.pub_key.value);
    let response = client
        .post(&url)
        .body(message)
        .header("content-type", "application/json")
        .header("X-Reservation-Signature", signature.signature)
        .send()
        .await?;
    log::info!("{:?} {}", response.status(), response.text().await?);
    Ok(())
}

fn parse_result(nft_contract: &str, tx: &TXResultBlock) -> ReservationTxResultRequest {
    if let Some(code) = tx.code {
        ReservationTxResultRequest {
            wallet_address: None,
            assigned_on: None,
            tx: tx.txhash.clone(),
            token_id: None,
            success: false,
            error: Some(format!(
                "{}/{}-{}",
                code,
                tx.codespace.as_ref().unwrap_or(&"".to_string()),
                tx.raw_log
            )),
        }
    } else {
        let wasm_events = tx.get_events("wasm");

        if let Some(wasm) = wasm_events.first() {
            let contract = wasm.get_first_value("contract_address").unwrap_or_default();
            if contract == nft_contract {
                let owner = wasm.get_first_value("minter").unwrap_or_default();
                let token_id = wasm.get_first_value("token_id").unwrap_or_default();

                ReservationTxResultRequest {
                    wallet_address: Some(owner),
                    assigned_on: Some(tx.timestamp),
                    tx: tx.txhash.clone(),
                    token_id: Some(token_id),
                    success: true,
                    error: None,
                }
            } else {
                ReservationTxResultRequest {
                    wallet_address: None,
                    assigned_on: None,
                    tx: tx.txhash.clone(),
                    token_id: None,
                    success: false,
                    error: Some(String::from("contract mismatch")),
                }
            }
        } else {
            ReservationTxResultRequest {
                wallet_address: None,
                assigned_on: None,
                tx: tx.txhash.clone(),
                token_id: None,
                success: false,
                error: Some(String::from("Unable to find event")),
            }
        }
    }
}
#[rocket::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let lcd = env::var("LCD_URL").expect("Missing LCD_URL environment variable");
    let chain = env::var("CHAIN_ID").expect("Missing CHAIN_ID environment variable");
    let nft_contract = env::var("NFT_CONTRACT").expect("Missing NFT_CONTRACT environment variable");
    let secp: Secp256k1<All> = Secp256k1::new();
    let signing_key_phrase = env::var("DEBUG_RESERVATION_AUTH")
        .expect("Environment Variable 'DEBUG_RESERVATION_AUTH' Not present");

    let signing_key = PrivateKey::from_words(&secp, &signing_key_phrase).unwrap();

    let terra: Terra = Terra::lcd_client_no_tx(&lcd, &chain).await.unwrap();
    if args.len() != 2 {
        eprintln!("Usage: {} <Reservation-Server>", args[0])
    } else {
        let server_url = args.get(1).expect("Requires a reservation-server-url");
        let c = Client::new();
        match c
            .get(format!("{}/reservation/in-process", server_url))
            .send()
            .await
        {
            Ok(response) => match response.json::<Vec<String>>().await {
                Ok(hashes) => {
                    log::info!("{}", hashes.join(", "));
                    for hash in hashes {
                        match terra.tx().get(&hash).await {
                            Ok(tx) => {
                                let reservation_result = parse_result(&nft_contract, &tx);
                                let _res = update_tx(
                                    &c,
                                    &secp,
                                    &signing_key,
                                    server_url,
                                    reservation_result,
                                )
                                .await;
                            }
                            Err(err) => {
                                log::error!("Terra TX {} Error - {:?}", hash, err);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                return;
            }
        }
    }
}
