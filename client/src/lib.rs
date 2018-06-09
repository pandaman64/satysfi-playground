#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;
use stdweb::web::event::*;
use stdweb::web::IEventTarget;
use stdweb::web::XmlHttpRequest;
use stdweb::Promise;
use stdweb::unstable::TryInto;

#[macro_use]
extern crate failure;
use failure::Error;

extern crate futures;
use futures::prelude::*;
use futures::channel::oneshot::channel;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate ot;
use ot::cs::Id;

type BaseOperation = ot::linewise::Operation;
type Operation = ot::selection::linewise::Operation<usize>;
type Client<C> = ot::cs::client::Client<Operation, C>;
type State = ot::cs::State<Operation>;

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct Query {
    since_id: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct Patch {
    id: Id,
    operation: Operation,
}
js_serializable!(Patch);
js_deserializable!(Patch);

#[derive(Debug, Fail)]
#[fail(display = "Error inside JS: {}", message)]
struct JSError {
    message: String,
}

impl JSError {
    fn new(s: String) -> Self {
        JSError { message: s }
    }
}

#[derive(Clone, Copy, Debug)]
struct AjaxConnection;

impl AjaxConnection {
    fn retrieve_id(&self) -> Result<String, JSError> {
        match js!{ return retrieve_id(); } {
            stdweb::Value::String(s) => Ok(s),
            _ => Err(JSError::new("cannot retrieve id of this session".into())),
        }
    }
}

fn get<R: serde::de::DeserializeOwned + 'static + std::fmt::Debug>(
    path: &str,
) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let (tx, rx) = channel();

    {
        let mut tx = Some(tx);
        let xhr_ = Rc::clone(&xhr);
        let _handle = xhr.add_event_listener::<ProgressLoadEvent, _>(move |_| {
            if let Ok(Some(s)) = xhr_.response_text() {
                if let Ok(value) = serde_json::from_str(&s) {
                    if let Some(tx) = tx.take() {
                        tx.send(value).unwrap();
                    }
                }
            }
        });
    }

    xhr.open("GET", path)
        .map_err(|_| JSError::new("XmlHttpRequest::open".into()))
        .and_then(|_| {
            xhr.send()
                .map_err(|_| JSError::new("XmlHttpRequest::send".into()))
        })
        .map_err(Into::into)
        .into_future()
        .and_then(|_| rx.map_err(Into::into))
}

fn patch<T: serde::Serialize + ?Sized, R: serde::de::DeserializeOwned + 'static + std::fmt::Debug>(
    path: &str,
    body: &T,
) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let (tx, rx) = channel();

    {
        let mut tx = Some(tx);
        let xhr_ = Rc::clone(&xhr);
        let _handle = xhr.add_event_listener::<ProgressLoadEvent, _>(move |_| {
            if let Ok(Some(s)) = xhr_.response_text() {
                if let Ok(value) = serde_json::from_str(&s) {
                    if let Some(tx) = tx.take() {
                        tx.send(value).unwrap();
                    }
                }
            }
        });
    }

    xhr.open("PATCH", path)
        .map_err(|_| JSError::new("XmlHttpRequest::open".into()))
        .map_err(Into::<Error>::into)
        .and_then(|_| xhr.set_request_header("Content-Type", "application/json").map_err(Into::into))
        .and_then(|_| serde_json::to_string(body).map_err(Into::into))
        .and_then(|body| {
            xhr.send_with_string(&body)
                .map_err(|_| JSError::new("XmlHttpRequest::send_with_string".into()).into())
        })
        .into_future()
        .and_then(|_| rx.map_err(Into::into))
}

#[derive(Debug, Fail)]
#[fail(display = "Error on ajax connection: {}", _0)]
struct AjaxConnectionError(Error);

impl From<Error> for AjaxConnectionError {
    fn from(e: Error) -> Self {
        AjaxConnectionError(e)
    }
}

impl ot::client::Connection<Operation> for AjaxConnection {
    type Error = AjaxConnectionError;
    type Output = Box<Future<Item = (Id, Operation), Error = Self::Error>>;
    type StateFuture = Box<Future<Item = State, Error = Self::Error>>;

    fn get_latest_state(&self) -> Self::StateFuture {
        Box::new(
            self.retrieve_id()
                .map_err(Into::into)
                .into_future()
                .and_then(move |id| get(&format!("/realtime/{}/latest", id)))
                .map_err(Into::into),
        )
    }

    fn get_patch_since(&self, since_id: &Id) -> Self::Output {
        let since_id = since_id.0;
        Box::new(
            self.retrieve_id()
                .map_err(Into::into)
                .into_future()
                .and_then(move |id| get(&format!("/realtime/{}/patch?since_id={}", id, since_id)))
                .map(|patch: Patch| (patch.id, patch.operation))
                .map_err(Into::into),
        )
    }

    fn send_operation(&self, base_id: Id, operation: Operation) -> Self::Output {
        let p = Patch {
            id: base_id,
            operation: operation,
        };

        Box::new(
            self.retrieve_id()
                .map_err(Into::<Error>::into)
                .map_err(Into::into)
                .into_future()
                .and_then(move |id| patch(&format!("/realtime/{}", id), &p))
                .map(|patch: Patch| (patch.id, patch.operation))
                .map_err(Into::into),
        )
    }
}

// TODO: allow multiple connection (is this really needed?)
thread_local! {
    static CONNECTION: AjaxConnection = AjaxConnection;
}

#[derive(Serialize, Deserialize)]
struct ConnectionHandle;
js_serializable!(ConnectionHandle);
js_deserializable!(ConnectionHandle);

thread_local! {
    static CLIENT: RefCell<Option<Client<AjaxConnection>>> = RefCell::new(None);
}

#[derive(Serialize, Deserialize)]
struct ClientHandle;
js_serializable!(ClientHandle);
js_deserializable!(ClientHandle);

#[derive(Serialize, Deserialize)]
struct OperationHandle(usize);
js_serializable!(OperationHandle);
js_deserializable!(OperationHandle);

// one random number for each session(user)
// (I hope this will not collide with others)
// TODO: rand 0.5 will support stdweb + wasm32-unknown-unknown, so let's use Uuid::new_v4() once
// uuid crate adopt rand 0.5
thread_local! {
    static USERID: Cell<Option<usize>> = Cell::new(None);
}

thread_local! {
    static OPERATIONS: RefCell<Vec<Operation>> = RefCell::new(vec![]);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Target(ot::selection::linewise::Target<usize>);
js_serializable!(Target);
js_deserializable!(Target);

fn get_user_id() -> usize {
    USERID.with(|user_id| user_id.get().unwrap())
}

#[js_export]
fn set_user_id(id: usize) {
    USERID.with(|user_id| user_id.set(Some(id)));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Position {
    row: usize,
    column: usize,
}
js_serializable!(Position);
js_deserializable!(Position);

fn get_line(document: stdweb::Reference, idx: usize) -> String {
    js!(
        return @{document}.getLine(@{idx as i32});
    ).try_into().unwrap()
}

fn get_lines(document: stdweb::Reference, start: usize, end: usize) -> Vec<String> {
    js!(
        return @{document}.getLines(@{start as i32}, @{end as i32});
    ).try_into().unwrap()
}

fn operate(_client: ClientHandle, op: BaseOperation) -> Option<OperationHandle> {
    CLIENT.with(|client| {
        // nll?
        let client = client.borrow();
        let client = client.as_ref().unwrap();
        let op = client.unsynced_content().ok()?.operate(op);
        OPERATIONS.with(|operations| {
            let mut operations = operations.borrow_mut();
            let handle = OperationHandle(operations.len());
            operations.push(op);
            Some(handle)
        })
    })
}

// end_middle_of_last_line means this insertion ends at the middle of the last line of the insertion
#[js_export]
fn insert(client: ClientHandle, start: usize, end: usize, length: usize, document: stdweb::Reference) -> Option<OperationHandle> {
    let mut op = BaseOperation::default();

    op.retain(start)
        .delete(1);

    if start == end {
        op.insert(get_line(document, start));
    } else {
        for line in get_lines(document, start, end).into_iter() {
            op.insert(line);
        }
    }

    op.retain(length - end - 1);

    operate(client, op)
}

#[js_export]
fn remove(client: ClientHandle, start: usize, end: usize, length: usize, document: stdweb::Reference) -> Option<OperationHandle> {
    let length = length + end - start;
    let mut op = BaseOperation::default();

    op.retain(start)
        .insert(get_line(document, start))
        .delete(end - start + 1)
        .retain(length - end - 1);

    operate(client, op)
}

fn to_rust_position(pos: Position, document: &stdweb::Reference) -> ot::selection::linewise::Position {
    use ot::selection::linewise::Position;
    if pos.column == 0 {
        return Position {
            row: pos.row,
            col: pos.column,
        };
    }

    let line: String = js!(return @{document}.getLine(@{pos.row as i32})).try_into().unwrap();
    let mut idx_16 = 0;
    let mut idx_8 = 0;

    for c in line.chars() {
        idx_16 += c.len_utf16();
        idx_8 += c.len_utf8();

        if idx_16 == pos.column {
            return Position {
                row: pos.row,
                col: idx_8,
            };
        }
    }

    unreachable!()
}

fn to_js_position(pos: ot::selection::linewise::Position, document: &stdweb::Reference) -> Position {
    if pos.col == 0 {
        return Position {
            row: pos.row,
            column: pos.col,
        };
    }

    let line: String = js!(return @{document}.getLine(@{pos.row as i32})).try_into().unwrap();
    let mut idx_16 = 0;
    let mut idx_8 = 0;

    for c in line.chars() {
        idx_16 += c.len_utf16();
        idx_8 += c.len_utf8();

        if idx_8 == pos.col {
            return Position {
                row: pos.row,
                column: idx_16,
            };
        }
    }

    unreachable!()
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Range {
    start: Position,
    end: Position,
}
js_serializable!(Range);
js_deserializable!(Range);

fn to_rust_range(range: Range, document: &stdweb::Reference) -> ot::selection::linewise::Selection {
    ot::selection::linewise::Selection::Range(to_rust_position(range.start, document), to_rust_position(range.end, document))
}

fn to_js_range(s: ot::selection::linewise::Selection, document: &stdweb::Reference) -> Range {
    match s {
        ot::selection::linewise::Selection::Cursor(pos) => {
            let pos = to_js_position(pos, document);
            Range {
                start: pos,
                end: pos,
            }
        },
        ot::selection::linewise::Selection::Range(start, end) => Range {
            start: to_js_position(start, document),
            end: to_js_position(end, document),
        },
    }
}

#[js_export]
fn select(_client: ClientHandle, cursor: Position, ranges: Vec<Range>, document: stdweb::Reference) -> Option<OperationHandle> {
    use ot::selection::linewise::Selection::Cursor;

    let mut selection = Vec::with_capacity(ranges.len() + 1);
    selection.push(Cursor(to_rust_position(cursor, &document)));
    selection.extend(ranges.into_iter().filter(|r| r.start != r.end).map(|r| to_rust_range(r, &document)));

    CLIENT.with(|client| {
        // nll?
        let mut client = client.borrow_mut();
        let client = client.as_mut().unwrap();

        let target = client.unsynced_content().ok()?;
        let mut new_selection = target.selection.clone();
        new_selection.insert(get_user_id(), selection);
        let op = target.select(new_selection);

        OPERATIONS.with(|operations| {
            let mut operations = operations.borrow_mut();
            let handle = OperationHandle(operations.len());
            operations.push(op);
            Some(handle)
        })
    })
}

#[js_export]
fn show_operation(op: OperationHandle) -> String {
    OPERATIONS.with(|operations| {
        let operations = operations.borrow();
        format!("{:?}", operations[op.0])
    })
}

#[js_export]
fn get_connection() -> ConnectionHandle {
    ConnectionHandle
}

#[js_export]
fn get_client(_connection: ConnectionHandle) -> Promise {
    Promise::from_future({
        CONNECTION.with(|connection| {
            Client::with_connection(*connection)
                .map(|client| {
                    CLIENT.with(|client_box| *client_box.borrow_mut() = Some(client));
                    ClientHandle
                })
                .map_err(|e| e.to_string())
        })
    })
}

#[js_export]
fn current_content(_client: ClientHandle) -> Option<Target> {
    CLIENT.with(
        |client| client.borrow().as_ref().unwrap().current_content().map(Target).ok()
    )
}

#[js_export]
fn unsynced_lines(_client: ClientHandle) -> Vec<String> {
    CLIENT.with(|client| {
        let client = client.borrow();
        let client = client.as_ref().unwrap();
        let content = client.unsynced_content().unwrap();
        content.base
    })
}

#[js_export]
fn unsynced_selection(_client: ClientHandle, document: stdweb::Reference) -> Vec<Range> {
    CLIENT.with(|client| {
        let client = client.borrow();
        let client = client.as_ref().unwrap();
        let content = client.unsynced_content().unwrap();
        content
            .selection[&get_user_id()]
            .iter()
            .map(|s| to_js_range(*s, &document))
            .collect()
    })
}

#[js_export]
fn push_operation(_client: ClientHandle, operation: OperationHandle) {
    CLIENT.with(|client| {
        OPERATIONS.with(|operations| {
            let operation = operations.borrow()[operation.0].clone();
            client
                .borrow_mut()
                .as_mut()
                .unwrap()
                .push_operation(operation);
        })
    })
}

#[js_export]
fn send_to_server(_client: ClientHandle) -> Option<Promise> {
    CLIENT.with(
        |client| match client.borrow_mut().as_mut().unwrap().send_to_server() {
            Ok(f) => Some(Promise::from_future(f.map(|(id, operation)| Patch {
                id,
                operation,
            }).map_err(|e| e.to_string()))),
            // TODO: log error or pass
            Err(_) => None,
        },
    )
}

/// if return value is non-null, this indicates the operation is not successful
#[js_export]
fn apply_response(_client: ClientHandle, patch: Patch) -> stdweb::Value {
    CLIENT.with(|client| {
        let mut client = client.borrow_mut();
        let client = client.as_mut().unwrap();
        match client.apply_response(patch.id, patch.operation) {
            Ok(()) => stdweb::Value::Null,
            Err(e) => stdweb::Value::String(e.to_string()),
        }
    })
}

#[js_export]
fn send_get_patch(_client: ClientHandle) -> Promise {
    CLIENT.with(|client| {
        let client = client.borrow();
        let client = client.as_ref().unwrap();
        Promise::from_future(
            client
                .send_get_patch()
                .map(|(id, operation)| Patch { id, operation })
                .map_err(|e| e.to_string()),
        )
    })
}

/// if return value is non-null, this indicates the operation is not successful
#[js_export]
fn apply_patch(_client: ClientHandle, patch: Patch) -> stdweb::Value {
    CLIENT.with(|client| {
        let mut client = client.borrow_mut();
        let client = client.as_mut().unwrap();
        match client.apply_patch(patch.id, patch.operation) {
            Ok(()) => stdweb::Value::Null,
            Err(e) => stdweb::Value::String(e.to_string()),
        }
    })
}

#[js_export]
fn initialize_stdweb() {
    stdweb::initialize()
}
