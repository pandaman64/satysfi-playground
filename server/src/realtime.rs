extern crate rocket;
use rocket::response::Redirect;
extern crate rocket_contrib;
use rocket_contrib::{Json, Template, UUID};

extern crate failure;
use failure::Error;

extern crate serde;
extern crate serde_json;

extern crate ot;
use realtime::ot::Operation;
use realtime::ot::server::Server;
use realtime::ot::util::{Id, State};

extern crate uuid;
use realtime::uuid::Uuid;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const BASE_PATH: &'static str = "tmp/realtime";

#[derive(FromForm, Clone, Copy, Debug)]
struct Query {
    since_id: usize,
}

#[derive(Serialize, Deserialize)]
struct ServerData {
    latest_pdf_name: String,
    server: Server,
}

struct ServerPool {
    servers: HashMap<Uuid, ServerData>,
}

impl ServerPool {
    fn retrieve_from_entry(entry: io::Result<fs::DirEntry>) -> Result<(Uuid, ServerData), Error> {
        let entry = entry?;
        let path = entry.path();
        let id = Uuid::parse_str(path.file_name().unwrap().to_str().unwrap())?;
        let file = File::open(&path)?;
        let data = serde_json::from_reader(file)?;

        Ok((id, data))
    }

    fn from_directory<P: AsRef<Path>>(path: P) -> ServerPool {
        let mut servers = HashMap::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries {
                match ServerPool::retrieve_from_entry(entry) {
                    Ok((id, data)) => {
                        servers.insert(id, data);
                    }
                    Err(e) => println!("from_directory: {}", e), // this should be debug! or something like it
                }
            }
        }

        ServerPool { servers }
    }

    fn get_data(&self, id: &Uuid) -> Option<&ServerData> {
        self.servers.get(id)
    }

    fn get(&self, id: &Uuid) -> Option<&Server> {
        self.servers.get(id).map(|data| &data.server)
    }

    fn get_mut(&mut self, id: &Uuid) -> Option<&mut Server> {
        self.servers.get_mut(id).map(|data| &mut data.server)
    }

    fn insert(&mut self, id: Uuid, data: ServerData) {
        self.servers.insert(id, data);
    }

    fn sync(&self) {
        let base = PathBuf::new().join(BASE_PATH);
        for (id, server) in self.servers.iter() {
            let path = base.join(&id.hyphenated().to_string());
            if let Ok(file) = File::create(path) {
                serde_json::to_writer(file, &server).unwrap();
            }
        }
    }
}

lazy_static! {
    static ref SERVER_POOL: RwLock<ServerPool> = RwLock::new(ServerPool::from_directory(BASE_PATH));
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
    #[fail(display = "Server with given id not found: {}", _0)]
    ServerNotFound(String),
}

#[get("/realtime/<id>/patch?<query>")]
fn get_patch(id: UUID, query: Query) -> Result<Json<Patch>, RealtimeError> {
    use self::RealtimeError::*;

    let pool = SERVER_POOL.read().unwrap();
    let server_data = pool.get(&id)
        .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;
    server_data
        .get_patch(&Id(query.since_id))
        .map_err(OT)
        .map(|(id, operation)| Json(Patch { id, operation }))
}

#[get("/realtime/<id>/latest")]
fn get_latest(id: UUID) -> Result<Json<State>, RealtimeError> {
    use self::RealtimeError::*;

    let pool = SERVER_POOL.read().unwrap();
    let server_data = pool.get(&id)
        .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;
    Ok(Json(server_data
        .current_state()
        .clone()))
}


#[get("/realtime/<id>")]
fn get_session(id: UUID) -> Result<Template, Error> {
    use self::RealtimeError::*;

    let pool = SERVER_POOL.read().unwrap();
    let data = pool.get_data(&id)
        .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;

    let mut ctx = HashMap::new();
    ctx.insert("pdfname", data.latest_pdf_name.clone());
    ctx.insert("id", id.hyphenated().to_string());

    Ok(Template::render("realtime", &ctx))
}

const DEFAULT_CODE: &'static str = "@require: stdjabook

document (|
  title = {\\SATySFi;概説};
  author = {Takashi SUWA};
  show-title = true;
  show-toc = false;
|) '<
    +p { Hello, \\SATySFi; Playground! }
>";

const DEFAULT_PDF: &'static str =
    "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a";

#[patch("/realtime/<id>", format = "application/json", data = "<patch>")]
fn patch_session(id: UUID, patch: Json<Patch>) -> Result<Json<Patch>, RealtimeError> {
    use realtime::RealtimeError::*;

    let ret;

    let mut pool = SERVER_POOL.write().unwrap();
    {
        let server = pool.get_mut(&id)
            .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;
        let patch = patch.into_inner();

        ret = server
            .modify(patch.id, patch.operation)
            .map(|(id, operation)| Json(Patch { id, operation }))
            .map_err(|e| OT(e))?;
    }

    pool.sync();

    Ok(ret)
}

#[get("/realtime/new")]
fn new_session() -> Result<Redirect, Error> {
    let id = Uuid::new_v4();
    let server = {
        let mut server = Server::new();
        let op = {
            let mut op = Operation::new();
            op.insert(DEFAULT_CODE.into());
            op
        };
        let initial_id = server.current_state().id.clone();
        server.modify(initial_id, op).unwrap();
        server
    };

    let mut pool = SERVER_POOL.write().unwrap();
    pool.insert(
        id,
        ServerData {
            latest_pdf_name: DEFAULT_PDF.into(),
            server,
        },
    );
    pool.sync();

    let id = id.hyphenated().to_string();
    let redirect_url = format!("/realtime/{}", id);

    Ok(Redirect::to(&redirect_url))
}
