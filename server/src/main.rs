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
use std::fs;
use std::fs::File;
use std::io::Write;

use std::error::Error;

#[get("/")]
fn index() -> NamedFile {
    NamedFile::open(Path::new("./index.html")).unwrap()
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
    fs::create_dir(&hash)?;

    let input_file_name = format!("{:x}/input.saty", hash);
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
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    {
        let mut stdout_file = File::create(&format!("{:x}/stdout", hash));
        let mut stderr_file = File::create(&format!("{:x}/stderr", hash));

        stdout_file.write_all(stdout.as_bytes())?;
        stderr_file.write_all(stderr.as_bytes())?;
    }
    
    Ok(Json(Output{
        name: filename,
        success: output.status.success(),
        stdout: stdout,
        stderr: stderr,
    }))
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, files, compile])
        .launch();
}