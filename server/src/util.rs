use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use failure::Error;

use sha2::Digest;

const BASE_PATH: &'static str = "tmp";

#[derive(Debug, Fail)]
#[fail(display = "invalid query: {}", message)]
struct QueryError {
    message: String,
}

fn retrieve_file<'a>(id: &'a str) -> Result<String, Error> {
    if id.len() != 64 {
        return Err(QueryError {
            message: "invalid length".into(),
        }.into());
    }
    for c in id.chars() {
        if !c.is_digit(16) {
            return Err(QueryError {
                message: "invalid character type".into(),
            }.into());
        }
    }

    let mut input_file = File::open(make_input_path(id))?;
    let mut content = String::new();
    input_file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn create_context(
    query: String,
    default_code: String,
    default_pdf: String,
) -> HashMap<&'static str, String> {
    if let Ok(s) = retrieve_file(&query) {
        let mut ret = HashMap::new();
        ret.insert("code", s);
        ret.insert("pdfname", query);
        return ret;
    }

    let mut ret = HashMap::new();
    ret.insert("code", default_code);
    ret.insert("pdfname", default_pdf);
    ret
}

pub fn make_input_dir<P: AsRef<Path>>(hash: P) -> PathBuf {
    Path::new(BASE_PATH).join(hash).join("input")
}

pub fn make_input_path<P: AsRef<Path>>(hash: P) -> PathBuf {
    make_input_dir(hash).join("input.saty")
}

pub fn make_output_dir<P: AsRef<Path>>(hash: P) -> PathBuf {
    Path::new(BASE_PATH).join(hash).join("output")
}

pub fn make_output_path<P: AsRef<Path>>(hash: P) -> PathBuf {
    make_output_dir(hash).join("output.pdf")
}
#[derive(Deserialize)]
pub struct Input {
    pub content: String,
}

#[derive(Serialize)]
pub struct Output {
    pub name: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Fail)]
#[fail(display = "Cache not found")]
struct CacheNotFound;

fn cache(hash: &str) -> Result<Output, Error> {
    let stdout_filename = make_input_dir(&hash).join("stdout");
    let stderr_filename = make_input_dir(&hash).join("stderr");

    if Path::new(BASE_PATH).join(&hash).is_dir() {
        let mut stdout_file = File::open(&stdout_filename)?;
        let mut stderr_file = File::open(&stderr_filename)?;
        let mut stdout = String::new();
        let mut stderr = String::new();

        stdout_file.read_to_string(&mut stdout)?;
        stderr_file.read_to_string(&mut stderr)?;

        Ok(Output {
            name: hash.into(),
            success: true,
            stdout: stdout,
            stderr: stderr,
        })
    } else {
        Err(CacheNotFound.into())
    }
}

pub async fn compile(input: String) -> Result<Output, Error> {
    use tokio::prelude::AsyncWriteExt;
    use tokio_process::CommandExt;

    let hash = sha2::Sha256::digest_str(&input);
    let hash = format!("{:x}", hash);
    let stdout_filename = make_input_dir(&hash).join("stdout");
    let stderr_filename = make_input_dir(&hash).join("stderr");

    if let Ok(output) = cache(&hash) {
        return Ok(output);
    }

    use tokio::fs::create_dir_all;
    tokio::await!(create_dir_all(make_input_dir(&hash)))?;
    tokio::await!(create_dir_all(make_output_dir(&hash)))?;

    let input_file_name = make_input_path(&hash);
    let mut input_file = tokio::await!(tokio::fs::File::create(input_file_name.clone()))?;
    tokio::await!(input_file.write_all_async(input.as_bytes()))?;

    let child = Command::new("./run.sh")
        .args(&[&input_file_name, &make_output_path(&hash)])
        //.env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn_async()?;

    let output = tokio::await!(child.wait_with_output())?;
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    {
        let mut stdout_file = tokio::await!(tokio::fs::File::create(stdout_filename))?;
        let mut stderr_file = tokio::await!(tokio::fs::File::create(stderr_filename))?;

        tokio::await!(stdout_file.write_all_async(stdout.as_bytes()))?;
        tokio::await!(stderr_file.write_all_async(stderr.as_bytes()))?;
    }

    Ok(Output {
        name: hash,
        success: output.status.success(),
        stdout: stdout,
        stderr: stderr,
    })
}
