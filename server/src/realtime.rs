extern crate rocket;
extern crate rocket_contrib;
use rocket_contrib::{Json, Template, UUID};

use std::path::PathBuf;
use std::io::Read;
use std::fs::File;
use std::fmt;
use std::error::Error;

extern crate serde;
extern crate serde_json;

extern crate ot;
use realtime::ot::server::Server;
use realtime::ot::util::Id;
use realtime::ot::Operation;

const BASE_PATH: &'static str = "tmp/realtime";

#[derive(FromForm, Clone, Copy, Debug)]
struct Query {
    since_id: usize,
}

fn retrieve_server_state(id: &UUID) -> Result<Server, Box<Error>> {
    let path = PathBuf::new().join(BASE_PATH).join(id.to_string());
    let file = File::open(path)?;
    let server = serde_json::from_reader(file)?;
    Ok(server)
}

#[derive(Serialize, Deserialize)]
struct Patch {
    id: Id,
    operation: Operation,
}

#[derive(Debug)]
enum RealtimeError {
    OT(String),
    RetrieveState(Box<Error>),
}

impl fmt::Display for RealtimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RealtimeError::*;

        match *self {
            OT(ref s) => write!(f, "RealtimeError::OT({})", s),
            RetrieveState(ref e) => write!(f, "RealtimeError::RetrieveState({})", e),
        }
    }
}

impl Error for RealtimeError {
    fn description(&self) -> &str {
        use self::RealtimeError::*;

        match *self {
            OT(ref s) => s,
            RetrieveState(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
        /*
        use self::RealtimeError::*;

        match *self {
            OT(_) => None,
            RetrieveState(ref e) => Some(e), // why can't we return e as &Error?
        }
        */
    }
}

#[get("/realtime/<id>/patch?<query>")]
fn get_patch(id: UUID, query: Query) -> Result<Json<Patch>, RealtimeError> {
    use self::RealtimeError::*;

    let server = retrieve_server_state(&id).map_err(RetrieveState)?;
    server.get_patch(&Id(query.since_id))
        .map_err(OT)
        .map(|(id, operation)| Json(Patch { id, operation }))
}

#[get("/realtime/<id>")]
fn get_session(id: UUID) -> Result<Template, Box<Error>> {
    use self::RealtimeError::*;

    let server = retrieve_server_state(&id).map_err(RetrieveState)?;

    unimplemented!()
}

#[derive(Deserialize)]
struct PatchResult;

#[patch("/realtime/<id>", format = "application/json", data = "<patch>")]
fn patch_session(id: UUID, patch: Json<Patch>) -> String {
    unimplemented!()
}

#[get("/realtime/new")]
fn new_session() -> Template {
    unimplemented!()
}

