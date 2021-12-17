use std::os::unix::prelude::OsStrExt;

use actix_rt::spawn;
use actix_web::{web, Responder};
use aws_sdk_s3::ByteStream;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{podman, Data};

#[derive(Deserialize)]
pub struct Request {
    /// The SATySFi source. We accept only UTF-8 encoded string.
    source: String,
}

#[derive(Serialize)]
pub struct Response {
    /// The status code of podman (and satysfi), if any.
    ///
    /// When the process is terminated by signal, this field is None. For more detail, please consult [`std::process::ExitStatus::code`].
    status: Option<i32>,
    /// Common part of the S3 URLs
    s3_url: String,
}

async fn put_objects(
    data: web::Data<Data>,
    build_id: &str,
    source: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    document: Option<Vec<u8>>,
) -> Result<(), actix_web::Error> {
    use actix_web::error::ErrorInternalServerError;
    const BUCKET: &str = "satysfi-playground";

    let source = spawn(
        data.s3_client
            .put_object()
            .bucket(BUCKET)
            .key(format!("{}/input.saty", build_id))
            .content_type("text/plain")
            .body(ByteStream::from(source.into_bytes()))
            .send(),
    );
    let stdout = spawn(
        data.s3_client
            .put_object()
            .bucket(BUCKET)
            .key(format!("{}/stdout.txt", build_id))
            .content_type("text/plain")
            .body(ByteStream::from(stdout))
            .send(),
    );
    let stderr = spawn(
        data.s3_client
            .put_object()
            .bucket(BUCKET)
            .key(format!("{}/stderr.txt", build_id))
            .content_type("text/plain")
            .body(ByteStream::from(stderr))
            .send(),
    );
    let document = document.map(|document| {
        spawn(
            data.s3_client
                .put_object()
                .bucket(BUCKET)
                .key(format!("{}/document.pdf", build_id))
                .content_type("application/pdf")
                .body(ByteStream::from(document))
                .send(),
        )
    });

    source
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorInternalServerError)?;
    stdout
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorInternalServerError)?;
    stderr
        .await
        .map_err(ErrorInternalServerError)?
        .map_err(ErrorInternalServerError)?;
    if let Some(document) = document {
        document
            .await
            .map_err(ErrorInternalServerError)?
            .map_err(ErrorInternalServerError)?;
    }

    Ok(())
}

#[actix_web::post("/persist")]
pub async fn post(
    request: web::Json<Request>,
    data: web::Data<Data>,
) -> Result<impl Responder, actix_web::Error> {
    let web::Json(request) = request;
    // Compute build_id as sha256(<SATySFi Docker version> + ":" + <source>).
    // We use hex-encoded build_id for the S3 key and the permalink URL.
    let build_id = {
        let mut hasher = Sha256::new();
        hasher.update(data.version.as_bytes());
        hasher.update(":");
        hasher.update(request.source.as_bytes());
        let build_id = hasher.finalize();
        hex::encode(build_id.as_slice())
    };

    let (output, document) =
        web::block(podman(request.source.clone(), data.clone())).await??;

    let s3_url = {
        put_objects(
            data.clone(),
            &build_id,
            request.source,
            output.stdout,
            output.stderr,
            document,
        )
        .await?;

        format!(
            "{}/{}",
            std::str::from_utf8(data.s3_public_endpoint.as_bytes()).unwrap(),
            build_id
        )
    };

    Ok(web::Json(Response {
        status: output.status.code(),
        s3_url,
    }))
}
