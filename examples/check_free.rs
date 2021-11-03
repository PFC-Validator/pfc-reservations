use anyhow::Error;
use dotenv::dotenv;
use pfc_reservation::requests::{AssignOwner, ReservationTxResultRequest};
use reqwest::Client;
use secp256k1::{All, Secp256k1};
use std::env;
use terra_rust_api::client::tx_types::TXResultBlock;
use terra_rust_api::{PrivateKey, Terra};

async fn assign_owner(
    client: &Client,
    secp: &Secp256k1<All>,
    signing_key: &PrivateKey,
    server_url: &str,
    status: ReservationTxResultRequest,
) -> Result<(), Error> {
    if status.success {
        let assign_owner = AssignOwner {
            token_id: status.token_id.unwrap(),
            wallet_address: status.wallet_address.unwrap(),
        };
        let message = serde_json::to_string(&assign_owner)?;
        let url = format!("{}/mint/assign-owner", server_url);
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
    }
    Ok(())
}

fn parse_result(nft_contract: &str, tx: &TXResultBlock, name: &str) -> ReservationTxResultRequest {
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
    } else if let Some(tx_block) = &tx.tx {
        for msg in &tx_block.value.msg {
            let contract = &msg.value.contract;
            if contract == nft_contract {
                let exec_msg = &msg.value.execute_msg;
                if let Some(obj) = exec_msg.as_object() {
                    if let Some(mint) = obj.get("mint") {
                        if let Some(mint_obj) = mint.as_object() {
                            let token_id = mint_obj
                                .get("token_id")
                                .map(|f| f.as_str())
                                .flatten()
                                .unwrap_or_default();
                            if token_id == name {
                                if let Some(owner) =
                                    mint_obj.get("owner").map(|f| f.as_str()).flatten()
                                {
                                    return ReservationTxResultRequest {
                                        wallet_address: Some(String::from(owner)),
                                        assigned_on: None,
                                        tx: tx.txhash.clone(),
                                        token_id: Some(String::from(token_id)),
                                        success: true,
                                        error: None,
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
        ReservationTxResultRequest {
            wallet_address: None,
            assigned_on: None,
            tx: tx.txhash.clone(),
            token_id: None,
            success: false,
            error: Some(String::from("Unable to find event")),
        }
    } else {
        ReservationTxResultRequest {
            wallet_address: None,
            assigned_on: None,
            tx: tx.txhash.clone(),
            token_id: None,
            success: false,
            error: Some(format!("Unable to find tx block")),
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
            .get(format!("{}/reservation/in-mint-process", server_url))
            .send()
            .await
        {
            Ok(response) => match response.json::<Vec<(String, String)>>().await {
                Ok(hashes) => {
                    //  log::info!("{}", hashes.join(", "));
                    for hash in hashes {
                        match terra.tx().get(&hash.0).await {
                            Ok(tx) => {
                                let reservation_result = parse_result(&nft_contract, &tx, &hash.1);

                                log::info!("{:?}", reservation_result);
                                if reservation_result.success {
                                    assign_owner(
                                        &c,
                                        &secp,
                                        &signing_key,
                                        &server_url,
                                        reservation_result,
                                    )
                                    .await
                                    .unwrap();
                                } else {
                                    log::warn!(
                                        "{} {} {:?}",
                                        hash.0,
                                        hash.1,
                                        reservation_result.error.unwrap_or_default()
                                    );
                                }
                            }
                            Err(err) => {
                                log::error!("Terra TX {} Error - {:?}", hash.0, err);
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
