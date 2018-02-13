#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

extern crate rocket;
use rocket::response::NamedFile;
use std::path::{Path, PathBuf};

extern crate rocket_contrib;
use rocket_contrib::Json;

extern crate sha2;
use sha2::Digest;

use std::process::{Command, Stdio};
use std::fs::File;
use std::io::Write;

use std::error::Error;

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/files/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("/tmp/satysfi-playground/").join(file)).ok()
}

#[derive(Deserialize)]
struct Input {
    content: String,
}

#[derive(Serialize)]
struct Output {
    name: String,
    success: bool,
    stdout: String,
    stderr: String,
}

#[post("/compile", format = "application/json", data = "<input>")]
fn compile(input: Json<Input>) -> Result<Json<Output>, Box<Error>> {
    let hash = sha2::Sha256::digest_str(&input.content);
    let input_file_name = format!("{:x}.saty", hash);
    let mut input_file = File::create(&input_file_name)?;
    input_file.write_all(input.content.as_bytes())?;

    let filename = format!("{:x}.pdf", hash);
    let child = Command::new("run.sh")
        .args(&[&input_file_name, &format!("/tmp/satysfi-playground/{}", filename)])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = child.wait_with_output()?;
    
    Ok(Json(Output{
        name: filename,
        success: output.status.success(),
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    }))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, files, compile])
        .launch();
}
