use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use failure::Error;

use sha2::Digest;

const BASE_PATH: &str = "tmp";

#[derive(Debug, Fail)]
#[fail(display = "invalid query: {}", message)]
struct QueryError {
    message: String,
}

fn retrieve_file(id: &str) -> Result<String, Error> {
    if id.len() != 64 {
        return Err(QueryError {
            message: format!("invalid length: len({}) == {}", id, id.len()),
        }
        .into());
    }
    for c in id.chars() {
        if !c.is_digit(16) {
            return Err(QueryError {
                message: "invalid character type".into(),
            }
            .into());
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
    match retrieve_file(&query) {
        Ok(s) => {
            let mut ret = HashMap::new();
            ret.insert("code", s);
            ret.insert("pdfname", query);
            log::info!("created context: {:?}", ret);
            return ret;
        }
        Err(e) => log::info!("create context failed: {:?}", e),
    }

    let mut ret = HashMap::new();
    ret.insert("code", default_code);
    ret.insert("pdfname", default_pdf);
    log::info!("default context: {:?}", ret);
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
        let mut stdout_file = File::open(stdout_filename)?;
        let mut stderr_file = File::open(stderr_filename)?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        stdout_file.read_to_string(&mut stdout)?;
        stderr_file.read_to_string(&mut stderr)?;

        Ok(Output {
            name: hash.into(),
            success: true,
            stdout,
            stderr,
        })
    } else {
        Err(CacheNotFound.into())
    }
}

pub async fn compile(input: &[u8]) -> Result<Output, Error> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use tokio_process::CommandExt;

    let hash = sha2::Sha256::digest(input);
    let hash = format!("{:x}", hash);
    let stdout_filename = make_input_dir(&hash).join("stdout");
    let stderr_filename = make_input_dir(&hash).join("stderr");

    if let Ok(output) = cache(&hash) {
        return Ok(output);
    }

    use std::fs::create_dir_all;
    create_dir_all(make_input_dir(&hash))?;
    create_dir_all(make_output_dir(&hash))?;

    let input_file_name = make_input_path(&hash);
    {
        let mut input_file = File::create(&input_file_name)?;
        input_file.write_all(&input)?;
        input_file.sync_all()?;
    }

    let create_output = tokio::await!(Command::new("docker")
        .args(&["create", "pandaman64/satysfi-playground"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output_async())?;

    failure::ensure!(
        create_output.status.success(),
        "creating container failed: {:?}",
        create_output.status
    );

    let container_name = {
        let mut result = create_output.stdout;
        assert_eq!(result.pop().unwrap(), b'\n');
        result
    };
    let input_name = {
        let mut copy = container_name.clone();
        copy.extend(b":/tmp/input.saty");
        copy
    };
    let output_name = {
        let mut copy = container_name.clone();
        copy.extend(b":/tmp/output.pdf");
        copy
    };

    let copy_input = tokio::await!(Command::new("docker")
        .args(&[
            "cp".as_ref() as &OsStr,
            input_file_name.as_ref(),
            OsStrExt::from_bytes(&input_name)
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status_async()?)?;

    failure::ensure!(copy_input.success(), "failed to copy input");

    let run_satysfi = tokio::await!(Command::new("timeout")
        .args(&[
            "-sKILL".as_ref() as &OsStr,
            "120s".as_ref(),
            "docker".as_ref(),
            "start".as_ref(),
            "-a".as_ref(),
            OsStrExt::from_bytes(&container_name)
        ])
        .output_async())?;

    if run_satysfi.status.success() {
        let copy_output = tokio::await!(Command::new("docker")
            .args(&[
                "cp".as_ref() as &OsStr,
                OsStrExt::from_bytes(&output_name),
                make_output_path(&hash).as_ref()
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status_async()?)?;

        failure::ensure!(copy_output.success(), "failed to copy output");
    }

    // it's ok to fail removing container now, we'll hopefully have periodic garbage collection
    if let Ok(remove_container) = Command::new("docker")
        .args(&[
            "rm".as_ref() as &OsStr,
            OsStrExt::from_bytes(&container_name),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn_async()
    {
        // import future 0.1
        use tokio::prelude::Future;

        // spawn clean up task instead of waiting for completion.
        // this task will wait until removal is done and then clean up OS resources
        tokio::spawn(remove_container.map(|_| ()).map_err(|_| ()));
    }

    {
        let mut stdout_file = File::create(stdout_filename)?;
        let mut stderr_file = File::create(stderr_filename)?;

        stdout_file.write_all(&run_satysfi.stdout)?;
        stderr_file.write_all(&run_satysfi.stderr)?;
    }

    Ok(Output {
        name: hash,
        success: run_satysfi.status.success(),
        stdout: String::from_utf8_lossy(&run_satysfi.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run_satysfi.stderr).into_owned(),
    })
}
