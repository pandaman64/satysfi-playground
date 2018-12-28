#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

use actix_web;
use actix_web::{server, App, HttpRequest, HttpResponse, fs::NamedFile, http, Json, ResponseError, middleware::Logger, fs::StaticFiles};
use std::path::PathBuf;

#[macro_use]
extern crate tera;
use tera::Tera;

use env_logger;

use std::fs::File;
use std::io::Read;

#[macro_use]
extern crate failure;

#[macro_use]
extern crate futures;

//mod realtime;
mod util;
use crate::util::*;

lazy_static! {
    static ref TEMPLATE: Tera = compile_templates!("templates/*.html");
}

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "Template Error: {}", _0)]
    TemplateError(String),
    #[fail(display = "IO Error: {}", _0)]
    IOError(std::io::Error),
    #[fail(display = "Compile Error")]
    CompileError,
    #[fail(display = "Uri Error: {}", _0)]
    UriSegmentError(actix_web::error::UriSegmentError),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError()
            .finish()
    }
}

const DEFAULT_CODE: &'static str = "@require: stdjabook

document (|
  title = {\\SATySFi;概説};
  author = {Takashi SUWA};
  show-title = true;
  show-toc = false;
|) '<
    +p { Hello, \\SATySFi; Playground! }
>";

const DEFAULT_PDF: &'static str =
    "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a";

fn permalink(query: String) -> Result<HttpResponse, Error> {
    let s = TEMPLATE
        .render(
            "index.html",
            &create_context(query, DEFAULT_CODE.into(), DEFAULT_PDF.into()),
        )
        .map_err(|e| Error::TemplateError(e.description().to_owned()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn index(_: HttpRequest) -> Result<HttpResponse, Error> {
    let s = TEMPLATE 
        .render(
            "index.html",
            &create_context(
                "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a".to_string(),
                DEFAULT_CODE.into(),
                DEFAULT_PDF.into(),
            ),
        )
        .map_err(|e| Error::TemplateError(e.description().to_owned()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

/*
// https://github.com/SergioBenitez/Rocket/issues/95#issuecomment-354824883
struct CachedFile(NamedFile);

impl<'a> response::Responder<'a> for CachedFile {
    fn respond_to(self, req: &rocket::Request) -> response::Result<'a> {
        response::Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", "max-age=86400") // a day
            .ok()
    }
}
*/

fn files(req: HttpRequest) -> Result<NamedFile, Error> {
    let hash: PathBuf = req.match_info()
        .query("hash")
        .map_err(Error::UriSegmentError)?;
    match NamedFile::open(make_output_path(&hash)) {
        Ok(file) => Ok(file),
        _ => File::open(make_input_path(&hash))
                .map_err(Error::IOError)
                .and_then(|mut f| {
                    let mut content = String::new();
                    f.read_to_string(&mut content)
                        .map_err(Error::IOError)?;
                    compile(content)
                        .map_err(|_| Error::CompileError)
                })
                .and_then(|output| NamedFile::open(output.name).map_err(Error::IOError)),
    }
}

fn compile_handler(input: Json<Input>) -> Result<Json<Output>, Error> {
    compile(input.content.to_string())
        .map(Json)
        .map_err(|_| Error::CompileError)
}

fn main() {
    env_logger::init();

    server::new(|| {
        App::new()
            .resource("/", |r| r.method(http::Method::GET).with(index))
            .handler("/assets", StaticFiles::new("./assets"))
            .resource("/files/{hash}", |r| r.method(http::Method::GET).with(files))
            .resource("/compile", |r| r.method(http::Method::POST).with(compile_handler))
            .resource("/permalink/{query}", |r| r.method(http::Method::GET).with(permalink)) 
            .middleware(Logger::default())
    })
    .bind("localhost:8000")
    .unwrap()
    .run();
}
