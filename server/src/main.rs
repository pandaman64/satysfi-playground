#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

extern crate rocket;
use rocket::response::NamedFile;
use std::path::PathBuf;

extern crate rocket_contrib;
use rocket_contrib::Json;
use rocket_contrib::Template;

use std::fs::File;
use std::io::Read;

extern crate sha2;

#[macro_use]
extern crate failure;
use failure::Error;

mod realtime;
mod util;
use util::*;

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

#[get("/permalink/<query>")]
fn permalink(query: String) -> Template {
    Template::render(
        "index",
        &create_context(query, DEFAULT_CODE.into(), DEFAULT_PDF.into()),
    )
}

#[get("/")]
fn index() -> Template {
    Template::render(
        "index",
        &create_context(
            "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a".to_string(),
            DEFAULT_CODE.into(),
            DEFAULT_PDF.into(),
        ),
    )
}

// for non-pdf files
#[get("/<path..>", rank = 2)]
fn assets(path: PathBuf) -> Option<NamedFile> {
    let path = PathBuf::new().join("assets").join(path);
    NamedFile::open(path).ok()
}

use rocket::response;
// https://github.com/SergioBenitez/Rocket/issues/95#issuecomment-354824883
struct CachedFile(NamedFile);

impl<'a> response::Responder<'a> for CachedFile {
    fn respond_to(self, req: &rocket::Request) -> response::Result<'a> {
        response::Response::build_from(self.0.respond_to(req)?)
            .raw_header("Cache-Control", "max-age=86400") // a day
            .ok()
    }
}

#[get("/files/<hash..>")]
fn files(hash: PathBuf) -> Option<CachedFile> {
    match NamedFile::open(make_output_path(&hash)) {
        Ok(file) => Some(file),
        _ => File::open(make_input_path(&hash))
            .ok()
            .and_then(|mut f| {
                let mut content = String::new();
                f.read_to_string(&mut content).ok()?;
                compile(content).ok()
            })
            .and_then(|output| NamedFile::open(output.name).ok()),
    }.map(CachedFile)
}

#[post("/compile", format = "application/json", data = "<input>")]
fn compile_handler(input: Json<Input>) -> Result<Json<Output>, Error> {
    compile(input.content.to_string()).map(Json)
}

fn main() {
    rocket::ignite()
        .mount(
            "/",
            routes![
                // basic functionalities
                index,
                assets,
                permalink,
                files,
                compile_handler,
                // for realtime editing
                realtime::get_session,
                realtime::get_patch,
                realtime::patch_session,
                realtime::new_session,
            ],
        )
        .attach(Template::fairing())
        .launch();
}
