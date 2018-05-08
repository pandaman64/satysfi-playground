#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;
use stdweb::web::event::*;
use stdweb::web::IEventTarget;
use stdweb::web::XmlHttpRequest;
use stdweb::Promise;

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

extern crate uuid;
use uuid::Uuid;

extern crate ot;
use ot::cs::Id;

type LineOperation = ot::charwise::Operation;
type BaseOperation = ot::linewise::Operation;
type Operation = ot::selection::linewise::Operation<Uuid>;
type Client<C> = ot::cs::client::Client<Operation, C>;
type State = ot::cs::State<Operation>;

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

#[derive(Clone, Serialize, Deserialize)]
struct UserId(Uuid);
js_serializable!(UserId);
js_deserializable!(UserId);

// one uuidv4 for each session(user)
thread_local! {
    static USERID: UserId = UserId(Uuid::new_v4());
}

thread_local! {
    static OPERATIONS: RefCell<Vec<Operation>> = RefCell::new(vec![]);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Target(ot::selection::linewise::Target<Uuid>);
js_serializable!(Target);
js_deserializable!(Target);

#[js_export]
fn get_user_id() -> UserId {
    USERID.with(|user_id| user_id.clone())
}

#[derive(Debug, Serialize, Deserialize)]
struct Position {
    row: usize,
    col: usize,
}
js_serializable!(Position);
js_deserializable!(Position);

#[derive(Debug, Serialize, Deserialize)]
struct Delta {
    action: String,
    start: Position,
    end: Position,
    lines: Vec<String>,
}
js_serializable!(Delta);
js_deserializable!(Delta);

#[js_export]
fn operate(_client: ClientHandle, js_delta: Delta, current_lines: Vec<String>) -> Option<()> {
    use std::borrow::Borrow;

    let op =
        match js_delta.action.borrow() {
            "insert" => {
                assert_eq!(js_delta.lines.len(), js_delta.end.row - js_delta.start.row + 1);

                let mut op = BaseOperation::default();

                op.retain(js_delta.start.row);

                if js_delta.start.row == js_delta.end.row {
                    let mut line = current_lines[js_delta.start.row].chars();
                    let mut line_op = LineOperation::default();
                    let mut idx = 0;

                    while let Some(c) = line.next() {
                        if idx == js_delta.start.col {
                            line_op.insert(js_delta.lines[0].clone());
                        } else if idx < js_delta.start.col || idx >= js_delta.end.col {
                            line_op.retain(c.len_utf8());
                        }

                        idx += c.len_utf16();
                    }

                    op.modify(line_op);
                } else if js_delta.start.row < js_delta.end.row { // start.row < end.row
                    // the first line
                    {
                        let mut line = current_lines[js_delta.start.row].chars();
                        let mut line_op = LineOperation::default();
                        let mut idx = 0;

                        loop {
                            if idx == js_delta.start.col {
                                line_op.insert(js_delta.lines[0].clone());
                                break;
                            }

                            if let Some(c) = line.next() {
                                line_op.retain(c.len_utf8());
                                idx += c.len_utf16();
                            } else {
                                unreachable!()
                            }
                        }
                        op.modify(line_op);
                    }

                    // middle lines
                    for line in js_delta.lines[1..(js_delta.lines.len() - 1)].iter() {
                        op.insert(line.clone());
                    }

                    // the last line
                    {
                        let insert_str = js_delta.lines.last().map(Clone::clone).unwrap();
                        let retain_len = current_lines[js_delta.end.row].len() - insert_str.len();
                        let mut line_op = LineOperation::default();
                        line_op.insert(insert_str);
                        line_op.retain(retain_len);
                        op.modify(line_op);
                    }
                } else {
                    unreachable!()
                }

                op.retain(current_lines.len() - js_delta.end.row - 1);

                op
            },
            "remove" => {
                let mut op = BaseOperation::default();

                op.retain(js_delta.start.row);

                if js_delta.start.row == js_delta.end.row {
                    assert_eq!(js_delta.lines.len(), 1);

                    let mut line = current_lines[js_delta.start.row].chars();
                    let mut line_op = LineOperation::default();
                    let mut idx = 0;

                    loop {
                        if idx == js_delta.start.col {
                            line_op.delete(js_delta.lines[0].len());
                        }

                        if let Some(c) = line.next() {
                            line_op.retain(c.len_utf8());
                            idx += c.len_utf16();
                        } else {
                            break;
                        }
                    }

                    op.modify(line_op);
                    op.retain(current_lines.len() - js_delta.end.row - 1);
                } else if js_delta.start.row < js_delta.end.row {
                    assert!(js_delta.lines.len() >= 2);

                    // merge the first line and the last line
                    {
                        let mut line = current_lines[js_delta.start.row].chars();
                        let mut line_op = LineOperation::default();
                        let mut idx = 0;

                        loop {
                            if idx == js_delta.start.col {
                                line_op.delete(js_delta.lines[0].len());
                                break;
                            }

                            if let Some(c) = line.next() {
                                line_op.retain(c.len_utf8());
                                idx += c.len_utf16();
                            } else {
                                unreachable!()
                            }
                        }

                        // last line
                        line_op.insert(line.collect());

                        op.modify(line_op);
                    }

                    // delete remaining lines
                    op.delete(js_delta.lines.len() - 1);
                } else {
                    unreachable!()
                }

                op
            },
            _ => return None,
        };
    
    CLIENT.with(|client| {
        // nll?
        let mut client = client.borrow_mut();
        let client = client.as_mut().unwrap();
        if let Ok(content) = client.current_content() {
            let op = content.operate(op);
            client.push_operation(op);
            Some(())
        } else {
            None
        }
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
