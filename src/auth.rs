use pfc_reservation::requests::ErrorResponse;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::serde::json::Json;
use rocket::Request;
use secp256k1::{All, Message, PublicKey, Secp256k1, Signature};
use sha2::{Digest, Sha256};
use std::env;
use terra_rust_api::PrivateKey;
use thread_local::ThreadLocal;

#[derive(Debug, Clone)]
pub enum SignatureError {
    MissingHeader,
}

#[derive(Debug)]
pub struct SignatureB64 {
    pub signature: String,
}

pub struct SignatureLocalStorage {
    pub secp: ThreadLocal<Secp256k1<All>>,
}
impl SignatureLocalStorage {
    pub fn new() -> Self {
        Self {
            secp: ThreadLocal::new(),
        }
    }
}
pub fn verify_signature(
    verify_string: &str,
    sig: &SignatureB64,
    public_key: &str,
) -> anyhow::Result<()> {
    let secp_tls = SignatureLocalStorage::new();
    let secp = secp_tls.secp.get_or(|| Secp256k1::new());
    let hash = Sha256::digest(verify_string.as_bytes());
    let hash_message: Message = Message::from_slice(&hash)?;
    let sig_bytes = base64::decode(&sig.signature)?;
    let sig_sec: Signature = Signature::from_compact(&sig_bytes)?;
    let pub_key_bytes = base64::decode(public_key)?;
    let pub_key: PublicKey = PublicKey::from_slice(&pub_key_bytes)?;
    Ok(secp.verify(&hash_message, &sig_sec, &pub_key)?)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SignatureB64 {
    type Error = SignatureError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Ok(debug) = env::var("DEBUG_IGNORE_SIG") {
            if debug == "true" {
                log::warn!("IGNORING SIGNATURES");
                return Outcome::Success(SignatureB64 {
                    signature: String::from("DEBUG IGNORE SIG TURNED ON"),
                });
            }
        }
        if let Some(sig) = request.headers().get_one("X-Reservation-Signature") {
            Outcome::Success(SignatureB64 {
                signature: String::from(sig),
            })
        } else {
            Outcome::Failure((Status::Forbidden, SignatureError::MissingHeader))
        }
    }
}
pub fn is_valid_address(wallet_address: &str) -> Result<(), Json<ErrorResponse>> {
    if wallet_address.len() != 44 || !wallet_address.starts_with("terra") {
        Err(Json(ErrorResponse {
            code: 404,
            message: "Expecting wallet address 'terra....'".into(),
        }))
    } else {
        Ok(())
    }
}

pub fn generate_signature(
    private_key: &PrivateKey,
    message: &str,
) -> Result<SignatureB64, Json<ErrorResponse>> {
    let secp_tls = SignatureLocalStorage::new();
    let secp = secp_tls.secp.get_or(|| Secp256k1::new());
    match private_key.sign(secp, message) {
        Ok(sig) => Ok(SignatureB64 {
            signature: sig.signature,
        }),
        Err(e) => Err(Json(ErrorResponse {
            code: 500,
            message: e.to_string(),
        })),
    }
}
