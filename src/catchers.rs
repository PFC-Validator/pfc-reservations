use pfc_reservation::requests::ErrorResponse;
use rocket::serde::json::Json;
use rocket::Catcher;

#[catch(500)]
fn internal_server_error() -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Internal server error".to_string(),
        code: 500,
    })
}
#[catch(404)]
fn not_found() -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Not Found".to_string(),
        code: 404,
    })
}
#[catch(422)]
fn malformed() -> Json<ErrorResponse> {
    Json(ErrorResponse {
        message: "Malformed Request".to_string(),
        code: 422,
    })
}

pub fn get_catchers() -> Vec<Catcher> {
    catchers![internal_server_error, not_found, malformed]
}
