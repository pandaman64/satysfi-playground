#[macro_use]
extern crate stdweb;
use stdweb::web::XmlHttpRequest;
use stdweb::web::IEventTarget;
use stdweb::web::event::*;

#[macro_use]
extern crate failure;
use failure::Error;

extern crate futures;
use futures::Future;
use futures::task::Context;
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

    fn poll(&mut self, _cx: &mut Context) -> Result<Async<Self::Item>, Self::Error> {
        use SubjectInner::*;

        match self.0.try_borrow_mut() {
            Ok(mut this) => 
                match std::mem::replace(&mut *this, Done) {
                    Pending => {
                        *this = Pending;
                        Ok(Async::Pending)
                    },
                    Error(e) => Err(e),
                    Ready(s) => Ok(Async::Ready(s)),
                    Done => unreachable!(),
                },
            Err(_) => Ok(Async::Pending),
        }
    }
}

struct AjaxConnection;

impl AjaxConnection {
    fn retrieve_id(&self) -> Result<String, JSError> {
        match js!(retrieve_id()) {
            stdweb::Value::String(s) => Ok(s),
            _ => Err(JSError::new("cannot retrieve id of this session".into()))
        }
    }
}

impl Connection for AjaxConnection {
    type Error = Error;
    type Output = Box<Future<Item = (Id, Operation), Error = Self::Error>>;
    type StateFuture = Box<Future<Item = State, Error = Self::Error>>;

    fn get_latest_state(&self) -> Self::StateFuture {
        let xhr = Rc::new(XmlHttpRequest::new());
        let result: Subject<State, Error> = Subject::new();

        {
            let xhr_ = xhr.clone();
            let mut result = result.clone();
            let _handle = xhr.add_event_listener::<ResourceLoadEvent, _>(move |_| {
                match xhr_.response_text() {
                    Ok(Some(s)) => 
                        match serde_json::from_str::<State>(&s) {
                            Ok(state) => result.ready(state),
                            Err(e) => result.error(InvalidResponse::new(format!("cannot decode json: {}", e))),
                        },
                    Ok(None) => result.error(InvalidResponse::new("response is empty".into())), 
                    Err(_) => result.error(JSError::new("XmlHttpRequest::response_text".into())),
                };
            });
        }


        let id = self.retrieve_id();
        Box::new(id.and_then(|id| 
            xhr.open("GET", &format!("/realtime/{}", id))
                .map_err(|_| JSError::new("XmlHttpRequest::open".into()))
                .and_then(|_| 
                    xhr.set_request_header("Content-Type", "application/json")
                        .map_err(|_| JSError::new("XmlHttpRequest::set_request_header".into()))
                ).and_then(|_|
                    xhr.send()
                        .map_err(|_| JSError::new("XmlHttpRequest::send_with_string".into()))
                ).map_err(Into::into)
            )
            .map_err(Into::into)
            .into_future()
            .and_then(|_| result))
    }

    fn get_patch_since(&self, since_id: &Id) -> Self::Output {
        // "/realtime/<session_id>/patch?since_id=<id>"
        unimplemented!()
    }

    fn send_operation(&self, base_id: Id, operation: Operation) -> Self::Output {
        unimplemented!()
    }
}

