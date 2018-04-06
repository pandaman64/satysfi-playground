extern crate rocket;
extern crate rocket_contrib;
use rocket_contrib::{Json, Template, UUID};

use std::path::PathBuf;
use std::fs::File;

extern crate failure;
use failure::Error;

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

fn retrieve_server_state(id: &UUID) -> Result<Server, Error> {
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

#[derive(Debug, Fail)]
enum RealtimeError {
    #[fail(display = "invalid operational transformation: {}", _0)] 
    OT(String),
    #[fail(display = "error on retrieving state: {}", _0)]
    RetrieveState(Error),
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
fn get_session(id: UUID) -> Result<Template, Error> {
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

