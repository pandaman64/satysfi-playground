extern crate rocket;
extern crate rocket_contrib;
use rocket_contrib::{Json, Template, UUID};

extern crate failure;
use failure::Error;

extern crate serde;
extern crate serde_json;

extern crate ot;
use realtime::ot::server::Server;
use realtime::ot::util::Id;
use realtime::ot::Operation;

extern crate uuid;
use realtime::uuid::Uuid;

use util::*;

use std::path::Path;
use std::fs;
use std::fs::File;
use std::io;
use std::collections::HashMap;
use std::sync::RwLock;

const BASE_PATH: &'static str = "tmp/realtime";

#[derive(FromForm, Clone, Copy, Debug)]
struct Query {
    since_id: usize,
}

struct ServerPool {
    servers: HashMap<Uuid, Server>,
}

impl ServerPool {
    fn from_directory<P: AsRef<Path>>(path: P) -> ServerPool {
        let mut servers = HashMap::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(id) = Uuid::parse_str(path.to_str().unwrap()) {
                            if let Ok(file) = File::open(&path) {
                                if let Ok(server) = serde_json::from_reader(file) {
                                    servers.insert(id, server);
                                }
                            }
                        }
                    }
                }
            }
        }

        ServerPool {
            servers
        }
    }

    fn get(&self, id: &Uuid) -> Option<&Server> {
        self.servers.get(id)
    }

    fn get_mut(&mut self, id: &Uuid) -> Option<&mut Server> {
        self.servers.get_mut(id)
    }

    fn insert(&mut self, id: Uuid, server: Server) {
        self.servers.insert(id, server);
    }

    fn sync(&self) -> Result<(), io::Error> {
        // TODO: implement
        Ok(())
    }
}

lazy_static! {
    static ref SERVER_POOL: RwLock<ServerPool> = RwLock::new(ServerPool::from_directory(BASE_PATH));
}

#[derive(Serialize, Deserialize)]
struct ServerData {
    pdfname: String,
    server: Server,
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
    let server_data = pool
        .get(&id)
        .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;
    server_data
        .get_patch(&Id(query.since_id))
        .map_err(OT)
        .map(|(id, operation)| Json(Patch { id, operation }))
}

#[get("/realtime/<id>")]
fn get_session(id: UUID) -> Result<Template, Error> {
    use self::RealtimeError::*;

    let pool = SERVER_POOL
        .read()
        .unwrap();
    let server = pool
        .get(&id)
        .ok_or_else(|| ServerNotFound(id.hyphenated().to_string()))?;

    unimplemented!()
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

const DEFAULT_PDF: &'static str = "9165b5e8141ca2457c13bf72fbf07f01e795ac5e3bb112f5ed01bc08fb9cbe1a";

#[derive(Deserialize)]
struct PatchResult;

#[patch("/realtime/<id>", format = "application/json", data = "<patch>")]
fn patch_session(id: UUID, patch: Json<Patch>) -> String {
    unimplemented!()
}

#[get("/realtime/new")]
fn new_session() -> Result<Template, Error> {
    let id = Uuid::new_v4();
    let server = {
        let mut server = Server::new();
        let op = {
            let mut op = Operation::new();
            op.insert(DEFAULT_CODE.into());
            op
        };
        let initial_id = server.current_state().id.clone();
        server.modify(initial_id, op);
        server
    };
    
    let mut pool = SERVER_POOL
        .write()
        .unwrap();
    pool.insert(id, server);
    pool.sync()?;

    let mut ctx = create_context(DEFAULT_PDF.into(), DEFAULT_CODE.into(), DEFAULT_PDF.into());
    ctx.insert("id", id.hyphenated().to_string());

    Ok(Template::render("realtime", &ctx))
}

