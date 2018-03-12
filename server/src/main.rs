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

use std::fmt;
use std::error::Error;

use std::collections::HashMap;

const base_path: &'static str = "tmp";

#[derive(Debug)]
struct QueryError {
    message: String
}

impl QueryError {
    fn new(message: String) -> Self {
        QueryError { message }
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("QueryError")
            .field("message", &self.message)
            .finish()
    }
}

impl Error for QueryError {
    fn description(&self) -> &str {
        &self.message
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

fn retrieve_file<'a>(id: &'a str) -> Result<String, Box<Error>> {
    if id.len() != 64 {
        return Err(Box::new(QueryError::new("invalid length".into())));
    }
    for c in id.chars() {
        if !c.is_digit(16) {
            return Err(Box::new(QueryError::new("invalid character type".into())));
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

fn make_input_dir<P: AsRef<Path>>(hash: P) -> PathBuf {
    Path::new(base_path).join(hash).join("input")
}

fn make_input_path<P: AsRef<Path>>(hash: P) -> PathBuf {
    make_input_dir(hash).join("input.saty")
}

fn make_output_dir<P: AsRef<Path>>(hash: P) -> PathBuf {
    Path::new(base_path).join(hash).join("output")
}

fn make_output_path<P: AsRef<Path>>(hash: P) -> PathBuf {
    make_output_dir(hash).join("output.pdf")
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

fn compile(input: String) -> Result<Output, Box<Error>> {
    let hash = sha2::Sha256::digest_str(&input);
    let hash = format!("{:x}", hash);
    let stdout_filename = make_input_dir(&hash).join("stdout");
    let stderr_filename = make_input_dir(&hash).join("stderr");

    if Path::new(base_path).join(&hash).is_dir() {
        let mut stdout_file = File::open(&stdout_filename)?;
        let mut stderr_file = File::open(&stderr_filename)?;
        let mut stdout = String::new();
        let mut stderr = String::new();

        stdout_file.read_to_string(&mut stdout)?;
        stderr_file.read_to_string(&mut stderr)?;

        return Ok(Output{
            name: hash,
            success: true,
            stdout: stdout,
            stderr: stderr,
        })
    }

    use std::fs::create_dir_all;
    println!("hoy");
    create_dir_all(make_input_dir(&hash))?;
    println!("hoy");
    create_dir_all(make_output_dir(&hash))?;

    let input_file_name = make_input_path(&hash);
    let mut input_file = File::create(&input_file_name)?;
    println!("hoy");
    input_file.write_all(input.as_bytes())?;

    println!("hoy");
    let child = Command::new("run.sh")
        .args(&[&input_file_name, &make_output_path(&hash)])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    println!("hoy");
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    println!("hoy");
    {
        let mut stdout_file = File::create(&stdout_filename)?;
        let mut stderr_file = File::create(&stderr_filename)?;

        stdout_file.write_all(stdout.as_bytes())?;
        stderr_file.write_all(stderr.as_bytes())?;
    }
    
    Ok(Output{
        name: hash,
        success: output.status.success(),
        stdout: stdout,
        stderr: stderr,
    })
}

#[post("/compile", format = "application/json", data = "<input>")]
fn compile_handler(input: Json<Input>) -> Result<Json<Output>, Box<Error>> {
    compile(input.content.to_string()).map(Json)
}

fn main() {
    rocket::ignite()
        .mount("/", routes![index, permalink, files, compile_handler])
        .attach(Template::fairing())
        .launch();
}
