use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::Deserialize;

#[derive(Deserialize)]
struct Req {
    foo: String,
}

#[actix_web::post("/hey")]
async fn test_lifetime(req: web::Json<Req>) -> impl Responder {
    req.foo.clone()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let factory = || App::new()
        .service(test_lifetime)
        .default_service(web::route().to(|| HttpResponse::NotFound().body("Hello, World!")));

    // systemd socket activationのときはHttpServer::listen(self, lst: TcpListener)を使えそう
    HttpServer::new(factory)
        .server_hostname("satysfi-playground.tech")
        .bind("localhost:8080")?
        .run()
        .await
}
