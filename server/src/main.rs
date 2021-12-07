use std::{
    convert::TryFrom,
    ffi::OsString,
    fs, io,
    os::unix::prelude::OsStrExt,
    process::{self, Command, Stdio},
};

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use aws_sdk_s3::{Client, Endpoint};
use http::Uri;

mod endpoint;

/// Application Data
#[derive(Debug, Clone)]
pub struct Data {
    /// The path to podman executable, "podman" by default
    podman: OsString,
    /// S3 API Endpoint
    s3_api_endpoint: OsString,
    /// S3 Public Access Endpoint
    s3_public_endpoint: OsString,
    /// S3 Client
    s3_client: Client,
    /// Version of the SATySFi Docker image. Used for computing build id.
    version: OsString,
}

/// Populate application data from environment variables
async fn populate_data() -> Data {
    let config = aws_config::load_from_env().await;
    let s3_api_endpoint = std::env::var_os("S3_API_ENDPOINT").unwrap();
    let s3_public_endpoint = std::env::var_os("S3_PUBLIC_ENDPOINT").unwrap();
    let s3_config = aws_sdk_s3::config::Builder::from(&config)
        .endpoint_resolver(Endpoint::immutable(
            Uri::try_from(s3_api_endpoint.as_bytes()).unwrap(),
        ))
        .build();
    let s3_client = Client::from_conf(s3_config);

    Data {
        podman: std::env::var_os("PODMAN").unwrap_or_else(|| OsString::from("podman")),
        s3_api_endpoint,
        s3_public_endpoint,
        s3_client,
        version: std::env::var_os("SATYSFI_DOCKER_VERSION")
            .unwrap_or_else(|| OsString::from("dev")),
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

    let data = web::Data::new(populate_data().await);

    let factory = move || {
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Compress::default())
            .service(endpoint::compile::post)
            .service(endpoint::persist::post)
            .default_service(web::route().to(|| HttpResponse::NotFound().body("Hello, World!")))
    };

    // systemd socket activationのときはHttpServer::listen(self, lst: TcpListener)を使えそう
    HttpServer::new(factory)
        .server_hostname("satysfi-playground.tech")
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
