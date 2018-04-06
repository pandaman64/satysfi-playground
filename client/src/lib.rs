#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::web::XmlHttpRequest;
use stdweb::web::IEventTarget;
use stdweb::web::event::*;
use stdweb::js_export;
use stdweb::Promise;

#[macro_use]
extern crate failure;
use failure::Error;

extern crate futures;
use futures::{Future, Poll, Async};
use futures::prelude::*;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate ot;
use ot::*;
use ot::util::*;
use ot::client::*;

use std::rc::Rc;
use std::cell::RefCell;

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
        JSError {
            message: s,
        }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Got invalid response: {}", message)]
struct InvalidResponse {
    message: String,
}

impl InvalidResponse {
    fn new(s: String) -> Self {
        InvalidResponse {
            message: s,
        }
    }
}

#[derive(Debug)]
enum SubjectInner<T, E> {
    Pending,
    Error(E),
    Ready(T),
    Done,
}

#[derive(Debug)]
struct Subject<T, E>(Rc<RefCell<SubjectInner<T, E>>>);

impl<T, E> std::clone::Clone for Subject<T, E> {
    fn clone(&self) -> Self {
        Subject(Rc::clone(&self.0))
    }
}

impl<T, E> Subject<T, E> {
    fn new() -> Self {
        Subject(Rc::new(RefCell::new(SubjectInner::Pending)))
    }

    fn ready<U: Into<T>>(&mut self, value: U) {
        use SubjectInner::*;
        match *self.0.borrow() {
            Error(_) | Ready(_) | Done => unreachable!(),
            _ => (),
        }

        *self.0.borrow_mut() = Ready(value.into());
    }

    fn error<U: Into<E>>(&mut self, error: U) {
        use SubjectInner::*;
        match *self.0.borrow() {
            Error(_) | Ready(_) | Done => unreachable!(),
            _ => (),
        }

        *self.0.borrow_mut() = Error(error.into());
    }
}

impl<T, E> Future for Subject<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use SubjectInner::*;

        match self.0.try_borrow_mut() {
            Ok(mut this) => 
                match std::mem::replace(&mut *this, Done) {
                    Pending => {
                        *this = Pending;
                        Ok(Async::NotReady)
                    },
                    Error(e) => Err(e),
                    Ready(s) => Ok(Async::Ready(s)),
                    Done => unreachable!(),
                },
            Err(_) => Ok(Async::NotReady),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct AjaxConnection;

impl AjaxConnection {
    fn retrieve_id(&self) -> Result<String, JSError> {
        match js!(retrieve_id()) {
            stdweb::Value::String(s) => Ok(s),
            _ => Err(JSError::new("cannot retrieve id of this session".into()))
        }
    }

}

fn get<R: serde::de::DeserializeOwned + 'static>(path: &str) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let result: Subject<R, Error> = Subject::new();

    {
        let xhr_ = Rc::clone(&xhr);
        let mut result = result.clone();
        let _handle = xhr.add_event_listener::<ResourceLoadEvent, _>(move |_| {
            match xhr_.response_text() {
                Ok(Some(s)) => 
                    match serde_json::from_str::<R>(&s) {
                        Ok(state) => result.ready(state),
                        Err(e) => result.error(InvalidResponse::new(format!("cannot decode json: {}", e))),
                    },
                Ok(None) => result.error(InvalidResponse::new("response is empty".into())), 
                Err(_) => result.error(JSError::new("XmlHttpRequest::response_text".into())),
            };
        });
    }

    xhr.open("GET", path)
        .map_err(|_| JSError::new("XmlHttpRequest::open".into()))
        .and_then(|_|
            xhr.send()
                .map_err(|_| JSError::new("XmlHttpRequest::send_with_string".into()))
        ).map_err(Into::into)
        .into_future()
        .and_then(|_| result)
}

fn post<T: serde::Serialize + ?Sized, R: serde::de::DeserializeOwned + 'static>(path: &str, body: &T) -> impl Future<Item = R, Error = Error> {
    let xhr = Rc::new(XmlHttpRequest::new());
    let result: Subject<R, Error> = Subject::new();

    {
        let xhr_ = Rc::clone(&xhr);
        let mut result = result.clone();
        let _handle = xhr.add_event_listener::<ResourceLoadEvent, _>(move |_| {
            match xhr_.response_text() {
                Ok(Some(s)) => 
                    match serde_json::from_str::<R>(&s) {
                        Ok(state) => result.ready(state),
                        Err(e) => result.error(InvalidResponse::new(format!("cannot decode json: {}", e))),
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
        .and_then(|body|
            xhr.send_with_string(&body)
                .map_err(|_| JSError::new("XmlHttpRequest::send_with_string".into()).into())
        )
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
        Box::new(self.retrieve_id()
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| get(&format!("/realtime/{}", id)))
            .map_err(Into::into))
    }

    fn get_patch_since(&self, since_id: &Id) -> Self::Output {
        let since_id = since_id.0;
        Box::new(self.retrieve_id()
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| get(&format!("/realtime/{}/patch?since_id={}", id, since_id)))
            .map_err(Into::into))
    }

    fn send_operation(&self, base_id: Id, operation: Operation) -> Self::Output {
        let patch = Patch {
            id: base_id,
            operation: operation,
        };

        Box::new(self.retrieve_id()
            .map_err(Into::<Error>::into)
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| post(&format!("/realtime/{}/patch", id), &patch))
            .map_err(Into::into))
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
fn retain(operation: OperationHandle, len: usize) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].retain(len);
        } else {
            unreachable!()
        }
    })
}

#[js_export]
fn insert(operation: OperationHandle, s: String) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].insert(s);
        } else {
            unreachable!()
        }
    })
}

#[js_export]
fn delete(operation: OperationHandle, len: usize) {
    OPERATIONS.with(|operations| {
        let mut operations = operations.borrow_mut();
        if operation.0 < operations.len() {
            operations[operation.0].delete(len);
        } else {
            unreachable!()
        }
    })
}

thread_local! {
    static OUTPUTFUTURES: RefCell<Vec<Option<<AjaxConnection as Connection>::Output>>> = RefCell::new(vec![]);
}

#[derive(Serialize, Deserialize)]
struct OutputFutureHandle(usize);
js_serializable!(OutputFutureHandle);
js_deserializable!(OutputFutureHandle);

#[js_export]
fn get_connection() -> ConnectionHandle {
    ConnectionHandle
}

#[js_export]
fn get_client(_connection: ConnectionHandle) -> Promise {
    Promise::from_future({
        CONNECTION.with(|connection| 
            Client::with_connection(*connection)
                .map(|client| {
                    CLIENT.with(|client_box| {
                        *client_box.borrow_mut() = Some(client)
                    });
                    ClientHandle
                })
                .map_err(|e| e.to_string())
        )
    })
}

#[js_export]
fn current_content(_client: ClientHandle) -> stdweb::Value {
    CLIENT.with(|client| 
        match client.borrow().as_ref().unwrap().current_content() {
            Ok(c) => stdweb::Value::String(c),
            // TODO: log error or pass
            Err(_) => stdweb::Value::Null,
        }
    )
}

#[js_export]
fn push_operation(_client: ClientHandle, operation: OperationHandle) {
    CLIENT.with(|client| 
        OPERATIONS.with(|operations| {
            let operation = operations.borrow()[operation.0].clone();
            client.borrow_mut().as_mut().unwrap().push_operation(operation);
        })
    )
}

#[js_export]
fn send_to_server(_client: ClientHandle) -> Option<OutputFutureHandle> {
    CLIENT.with(|client| 
        match client.borrow_mut().as_mut().unwrap().send_to_server() {
            Ok(f) => {
                OUTPUTFUTURES.with(|output_futures| {
                    let mut output_futures = output_futures.borrow_mut();
                    let handle = OutputFutureHandle(output_futures.len());
                    output_futures.push(Some(f));
                    Some(handle)
                })
            },
            // TODO: log error or pass
            Err(_) => None 
        }
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
fn send_get_patch(_client: ClientHandle) -> OutputFutureHandle {
    CLIENT.with(|client| {
        let client = client.borrow();
        let client = client.as_ref().unwrap();
        OUTPUTFUTURES.with(|output_futures| {
            let mut output_futures = output_futures.borrow_mut();
            let handle = OutputFutureHandle(output_futures.len());
            output_futures.push(Some(Box::new(client.send_get_patch().map_err(Into::<Error>::into).map_err(Into::into))));
            handle
        })
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
            Err(e) => stdweb::Value::String(e.to_string())
        }
    })
}
