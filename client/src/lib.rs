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

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate ot;
use ot::client::*;
use ot::util::*;
use ot::*;

use std::cell::RefCell;
use std::rc::Rc;

extern crate subject;
use subject::Subject;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
struct Query {
    since_id: usize,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug, Fail)]
#[fail(display = "Got invalid response: {}", message)]
struct InvalidResponse {
    message: String,
}

impl InvalidResponse {
    fn new(s: String) -> Self {
        InvalidResponse { message: s }
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

fn get<R: serde::de::DeserializeOwned + 'static>(
    path: &str,
) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let result: Subject<R, Error> = Subject::new();

    {
        let xhr_ = Rc::clone(&xhr);
        let mut result = result.clone();
        let _handle = xhr.add_event_listener::<ProgressLoadEvent, _>(move |_| {
            match xhr_.response_text() {
                Ok(Some(s)) => match serde_json::from_str::<R>(&s) {
                    Ok(state) => result.ready(state),
                    Err(e) => {
                        result.error(InvalidResponse::new(format!("cannot decode json: {}", e)))
                    }
                },
                Ok(None) => result.error(InvalidResponse::new("response is empty".into())),
                Err(_) => result.error(JSError::new("XmlHttpRequest::response_text".into())),
            };
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
        .and_then(|_| result)
}

fn post<T: serde::Serialize + ?Sized, R: serde::de::DeserializeOwned + 'static>(
    path: &str,
    body: &T,
) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let result: Subject<R, Error> = Subject::new();

    {
        let xhr_ = Rc::clone(&xhr);
        let mut result = result.clone();
        let _handle = xhr.add_event_listener::<ProgressLoadEvent, _>(move |_| {
            match xhr_.response_text() {
                Ok(Some(s)) => match serde_json::from_str::<R>(&s) {
                    Ok(state) => result.ready(state),
                    Err(e) => {
                        result.error(InvalidResponse::new(format!("cannot decode json: {}", e)))
                    }
                },
                Ok(None) => result.error(InvalidResponse::new("response is empty".into())),
                Err(_) => result.error(JSError::new("XmlHttpRequest::response_text".into())),
            };
        });
    }

    xhr.open("GET", path)
        .map_err(|_| JSError::new("XmlHttpRequest::open".into()))
        .map_err(Into::<Error>::into)
        .and_then(|_| serde_json::to_string(body).map_err(Into::into))
        .and_then(|body| {
            xhr.send_with_string(&body)
                .map_err(|_| JSError::new("XmlHttpRequest::send_with_string".into()).into())
        })
        .into_future()
        .and_then(|_| result)
}

#[derive(Debug, Fail)]
#[fail(display = "Error on ajax connection: {}", _0)]
struct AjaxConnectionError(Error);

impl From<Error> for AjaxConnectionError {
    fn from(e: Error) -> Self {
        AjaxConnectionError(e)
    }
}

impl Connection for AjaxConnection {
    type Error = AjaxConnectionError;
    type Output = Box<Future<Item = (Id, Operation), Error = Self::Error>>;
    type StateFuture = Box<Future<Item = State, Error = Self::Error>>;

    fn get_latest_state(&self) -> Self::StateFuture {
        Box::new(
            self.retrieve_id()
                .map_err(Into::into)
                .into_future()
                .and_then(move |id| get(&format!("/realtime/{}", id)))
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
                .map_err(Into::into),
        )
    }

    fn send_operation(&self, base_id: Id, operation: Operation) -> Self::Output {
        let patch = Patch {
            id: base_id,
            operation: operation,
        };

        Box::new(
            self.retrieve_id()
                .map_err(Into::<Error>::into)
                .map_err(Into::into)
                .into_future()
                .and_then(move |id| post(&format!("/realtime/{}/patch", id), &patch))
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

thread_local! {
    static OPERATIONS: RefCell<Vec<Operation>> = RefCell::new(vec![]);
}

#[js_export]
fn new_operation() -> OperationHandle {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        let handle = OperationHandle(operations.len());
        operations.push(Operation::new());
        handle
    })
}

#[js_export]
fn show_operation(operation: OperationHandle) -> Option<String> {
    OPERATIONS.with(|operations| {
        let operations = operations.borrow();
        operations
            .get(operation.0)
            .map(|op| format!("{:?}", op))
    })
}

fn retain(operation: OperationHandle, len: usize) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].retain(len);
        } else {
            unreachable!("operation out of range")
        }
    })
}

#[js_export]
fn retain_str(operation: OperationHandle, s: &str) {
    retain(operation, s.len())
}

#[js_export]
fn insert(operation: OperationHandle, s: String) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].insert(s);
        } else {
            unreachable!("operation out of range")
        }
    })
}

fn delete(operation: OperationHandle, len: usize) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].delete(len);
        } else {
            unreachable!("operation out of range")
        }
    })
}

#[js_export]
fn delete_str(operation: OperationHandle, s: &str) {
    delete(operation, s.len())
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
fn current_content(_client: ClientHandle) -> stdweb::Value {
    CLIENT.with(
        |client| match client.borrow().as_ref().unwrap().current_content() {
            Ok(c) => stdweb::Value::String(c),
            // TODO: log error or pass
            Err(_) => stdweb::Value::Null,
        },
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
