#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

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

use std::collections::HashMap;

mod realtime;
mod util;
use util::*;

#[derive(Debug, Fail)]
#[fail(display = "invalid query: {}", message)]
struct QueryError {
    message: String
}

fn retrieve_file<'a>(id: &'a str) -> Result<String, Error> {
    if id.len() != 64 {
        return Err(QueryError { message: "invalid length".into() }.into());
    }
    for c in id.chars() {
        if !c.is_digit(16) {
            return Err(QueryError { message: "invalid character type".into() }.into());
        }
    }

    let mut input_file = File::open(make_input_path(id))?;
    let mut content = String::new();
    input_file.read_to_string(&mut content)?;
    Ok(content)
}

fn create_context(query: String) -> HashMap<&'static str, String> {
    if let Ok(s) = retrieve_file(&query) {
        let mut ret = HashMap::new();
        ret.insert("code", s);
        ret.insert("pdfname", query);
        return ret;
    }

    let mut ret = HashMap::new();
    ret.insert("code", "@require: stdjabook

document (|
  title = {\\SATySFi;概説};
  author = {Takashi SUWA};
  show-title = true;
  show-toc = false;
|) '<
    +p { Hello, \\SATySFi; Playground! }
>".to_string());
    ret.insert("pdfname", "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a".to_string());
    ret
}

#[get("/permalink/<query>")]
fn permalink(query: String) -> Template {
    Template::render("index", &create_context(query))
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &create_context("9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a".to_string()))
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
                .and_then(|output| NamedFile::open(output.name).ok())
    }.map(CachedFile)
}

#[post("/compile", format = "application/json", data = "<input>")]
fn compile_handler(input: Json<Input>) -> Result<Json<Output>, Error> {
    compile(input.content.to_string()).map(Json)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![
               // basic functionalities
               index,
               permalink,
               files,
               compile_handler,
               // for realtime editing
               realtime::get_session,
               realtime::patch_session,
               realtime::new_session,
        ])
        .attach(Template::fairing())
        .launch();
}
