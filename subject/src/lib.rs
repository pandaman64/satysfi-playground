extern crate futures;
use futures::{Async, Future, Poll};

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
enum SubjectInner<T, E> {
    Pending,
    Error(E),
    Ready(T),
    Done,
}

#[derive(Debug)]
pub struct Subject<T, E>(Rc<RefCell<SubjectInner<T, E>>>);

impl<T, E> std::clone::Clone for Subject<T, E> {
    fn clone(&self) -> Self {
        Subject(Rc::clone(&self.0))
    }
}

impl<T, E> Subject<T, E> {
    pub fn new() -> Self {
        Subject(Rc::new(RefCell::new(SubjectInner::Pending)))
    }

    pub fn ready<U: Into<T>>(&mut self, value: U) {
        use SubjectInner::*;
        match *self.0.borrow() {
            Error(_) | Ready(_) | Done => unreachable!("error setting ready: already set"),
            _ => (),
        }

        *self.0.borrow_mut() = Ready(value.into());
    }

    pub fn error<U: Into<E>>(&mut self, error: U) {
        use SubjectInner::*;
        match *self.0.borrow() {
            Error(_) | Ready(_) | Done => unreachable!("error setting error: already set"),
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
            Ok(mut this) => match std::mem::replace(&mut *this, Done) {
                Pending => {
                    *this = Pending;
                    Ok(Async::NotReady)
                }
                Error(e) => Err(e),
                Ready(s) => Ok(Async::Ready(s)),
                Done => unreachable!("polling is already done"),
            },
            Err(_) => Ok(Async::NotReady),
        }
    }
}

#[test]
fn test_subject() {
    {
        let mut subject = Subject::<i32, String>::new();
        assert_eq!(subject.poll(), Ok(Async::NotReady));
        subject.ready(42);
        assert_eq!(subject.poll(), Ok(Async::Ready(42)));
    }

    {
        let mut subject = Subject::<i32, String>::new();
        assert_eq!(subject.poll(), Ok(Async::NotReady));
        subject.error("error");
        assert_eq!(subject.poll(), Err("error".into()));
    }
}
