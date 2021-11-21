use std::{
    ffi::OsString,
    fs, io,
    process::{self, Command, Stdio},
};

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CompileRequest {
    /// The SATySFi source in base64-encoded string.
    source: String,
}

#[derive(Serialize)]
struct CompileResponse {
    /// The status code of podman (and satysfi), if any.
    ///
    /// When the process is terminated by signal, this field is None. For more detail, please consult [`std::process::ExitStatus::code`].
    status: Option<i32>,
    /// The stdout of satysfi in base64-encoded string.
    stdout: String,
    /// The stderr of satysfi in base64-encoded string.
    stderr: String,
    /// The PDF file generated by satysfi in base64-encoded string.
    document: Option<String>,
}

#[actix_web::post("/compile")]
async fn compile(request: web::Json<CompileRequest>) -> Result<impl Responder, actix_web::Error> {
    let (output, document) =
        web::block(move || -> io::Result<(process::Output, Option<Vec<u8>>)> {
            let dir = tempfile::tempdir()?;
            let input_path = dir.path().join("input.saty");
            fs::write(
                input_path,
                base64::decode(&request.source)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            )?;

            let output = Command::new("podman")
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
        })
        .await??;

    Ok(web::Json(CompileResponse {
        status: output.status.code(),
        stdout: base64::encode(output.stdout),
        stderr: base64::encode(output.stderr),
        document: document.map(base64::encode),
    }))
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let factory = || {
        App::new()
            .service(compile)
            .default_service(web::route().to(|| HttpResponse::NotFound().body("Hello, World!")))
    };

    // systemd socket activationのときはHttpServer::listen(self, lst: TcpListener)を使えそう
    HttpServer::new(factory)
        .server_hostname("satysfi-playground.tech")
        .bind("localhost:8080")?
        .run()
        .await
}
