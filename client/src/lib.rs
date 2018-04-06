#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;
use stdweb::web::XmlHttpRequest;
use stdweb::web::IEventTarget;
use stdweb::web::event::*;

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
        Subject(self.0.clone())
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
        let xhr_ = xhr.clone();
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
        let xhr_ = xhr.clone();
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

impl Connection for AjaxConnection {
    type Error = Error;
    type Output = Box<Future<Item = (Id, Operation), Error = Self::Error>>;
    type StateFuture = Box<Future<Item = State, Error = Self::Error>>;

    fn get_latest_state(&self) -> Self::StateFuture {
        Box::new(self.retrieve_id()
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| get(&format!("/realtime/{}", id))))
    }

    fn get_patch_since(&self, since_id: &Id) -> Self::Output {
        let since_id = since_id.0;
        Box::new(self.retrieve_id()
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| get(&format!("/realtime/{}/patch?since_id={}", id, since_id))))
    }

    fn send_operation(&self, base_id: Id, operation: Operation) -> Self::Output {
        let patch = Patch {
            id: base_id,
            operation: operation,
        };

        Box::new(self.retrieve_id()
            .map_err(Into::into)
            .into_future()
            .and_then(move |id| post(&format!("/realtime/{}/patch", id), &patch)))
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

#[js_export]
fn get_connection() -> ConnectionHandle {
    ConnectionHandle
}

#[js_export]
fn get_client(_connection: ConnectionHandle) -> stdweb::Promise {
        stdweb::Promise::from_future({
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

