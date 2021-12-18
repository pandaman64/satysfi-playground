#[actix_web::get("/healthcheck")]
pub async fn get() -> &'static str {
    "OK"
}
