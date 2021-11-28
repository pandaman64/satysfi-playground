use std::{
    ffi::OsString,
    fs, io,
    process::{self, Command, Stdio},
};

use actix_web::{middleware, web, App, HttpResponse, HttpServer};

mod endpoint;

/// Application Data
#[derive(Debug, Clone)]
pub struct Data {
    /// The path to podman executable, "podman" by default
    podman: OsString,
}

/// Populate application data from environment variables
fn populate_data() -> Data {
    Data {
        podman: std::env::var_os("PODMAN").unwrap_or_else(|| OsString::from("podman")),
    }
}

/// Return a function that runs podman command to compile a SATySFi source
fn podman(
    source: String,
    data: web::Data<Data>,
) -> impl FnOnce() -> io::Result<(process::Output, Option<Vec<u8>>)> {
    move || {
        let dir = tempfile::tempdir()?;
        let input_path = dir.path().join("input.saty");
        fs::write(
            input_path,
            base64::decode(&source).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
        )?;

        let output = Command::new(&data.podman)
            .arg("run")
            .arg("--rm")
            .arg({
                // format!("--volume={}:/tmp", dir.path()) without conversion from/to String
                let mut buffer = OsString::from("--volume=");
                buffer.push(dir.path());
                buffer.push(":/tmp");
                buffer
            })
            .arg("satysfi:latest")
            .stdin(Stdio::null())
            .output()?;
        let document = if output.status.success() {
            let document_path = dir.path().join("output.pdf");
            let document = fs::read(document_path)?;
            Some(document)
        } else {
            None
        };

        // TODO: close the temporary directory and emit log if an error occurs
        Ok((output, document))
    }
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let data = web::Data::new(populate_data());

    let factory = move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Compress::default())
            .service(endpoint::compile::post)
            .default_service(web::route().to(|| HttpResponse::NotFound().body("Hello, World!")))
    };

    // systemd socket activationのときはHttpServer::listen(self, lst: TcpListener)を使えそう
    HttpServer::new(factory)
        .server_hostname("satysfi-playground.tech")
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
