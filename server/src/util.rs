use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

extern crate failure;
use failure::Error;

extern crate sha2;
use sha2::Digest;

const BASE_PATH: &'static str = "tmp";

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

pub fn compile(input: String) -> Result<Output, Error> {
    let hash = sha2::Sha256::digest_str(&input);
    let hash = format!("{:x}", hash);
    let stdout_filename = make_input_dir(&hash).join("stdout");
    let stderr_filename = make_input_dir(&hash).join("stderr");

    if Path::new(BASE_PATH).join(&hash).is_dir() {
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
    create_dir_all(make_input_dir(&hash))?;
    create_dir_all(make_output_dir(&hash))?;

    let input_file_name = make_input_path(&hash);
    let mut input_file = File::create(&input_file_name)?;
    input_file.write_all(input.as_bytes())?;

    let child = Command::new("run.sh")
        .args(&[&input_file_name, &make_output_path(&hash)])
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
    
    Ok(Output{
        name: hash,
        success: output.status.success(),
        stdout: stdout,
        stderr: stderr,
    })
}
