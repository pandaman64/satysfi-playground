#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate serde_derive;

extern crate rocket;
use rocket::response::NamedFile;
use std::path::{Path, PathBuf};

extern crate rocket_contrib;
use rocket_contrib::Json;
use rocket_contrib::Template;

extern crate sha2;
use sha2::Digest;

use std::process::{Command, Stdio};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

use std::error::Error;

use std::collections::HashMap;

#[derive(FromForm, Debug)]
struct IndexQuery {
    id: String,
}

#[get("/?<query>")]
fn index_query(query: Option<IndexQuery>) -> Template {
    println!("{:?}", query);
    let context = {
        let mut m = HashMap::new();
        m.insert("pdfname", query.map_or("2f4b1088a4526a5faf4dea3c3ca6940113247c550951e1ecc74e510ff5ab689b.pdf".to_string(), |q| q.id));
        m
    };
    Template::render("index", &context)
}

#[get("/")]
fn index() -> Template {
    index_query(None)
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
    let hash = format!("{:x}", hash);
    let filename = format!("{}.pdf", hash);
    let stdout_filename = format!("{}/stdout", hash);
    let stderr_filename = format!("{}/stderr", hash);

    if Path::new(&hash).is_dir() {
        let mut stdout_file = File::open(&stdout_filename)?;
        let mut stderr_file = File::open(&stderr_filename)?;
        let mut stdout = String::new();
        let mut stderr = String::new();

        stdout_file.read_to_string(&mut stdout)?;
        stderr_file.read_to_string(&mut stderr)?;

        return Ok(Json(Output{
            name: filename,
            success: true,
            stdout: stdout,
            stderr: stderr,
        }))
    }
    fs::create_dir(&hash)?;

    let input_file_name = format!("{}/input.saty", hash);
    let mut input_file = File::create(&input_file_name)?;
    input_file.write_all(input.content.as_bytes())?;

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
        let mut stdout_file = File::create(&stdout_filename)?;
        let mut stderr_file = File::create(&stderr_filename)?;

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
        .mount("/", routes![index, index_query, files, compile])
        .attach(Template::fairing())
        .launch();
}
