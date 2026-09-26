//! A promise has one producer result and independently pollable waiters.
//!
//! The unpolled anchor retains completion even when a waiter times out or is
//! dropped. No receiver is replaced by a closed dummy channel during an await.
use super::Value;
use futures::future::Shared;
use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot::{self, error::TryRecvError};

type Receiver = oneshot::Receiver<Result<Value, String>>;
type Completion = Result<Result<Value, String>, oneshot::error::RecvError>;

pub struct PromiseReceiver {
    anchor: Shared<Receiver>,
    waiter: Option<Shared<Receiver>>,
}

impl From<Receiver> for PromiseReceiver {
    fn from(receiver: Receiver) -> Self {
        Self { anchor: receiver.shared(), waiter: None }
    }
}

impl Clone for PromiseReceiver {
    fn clone(&self) -> Self {
        Self { anchor: self.anchor.clone(), waiter: None }
    }
}

impl Future for PromiseReceiver {
    type Output = Completion;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(result) = this.anchor.peek() {
            return Poll::Ready(result.clone());
        }
        let waiter = this.waiter.get_or_insert_with(|| this.anchor.clone());
        let result = Pin::new(waiter).poll(cx);
        if result.is_ready() {
            this.waiter = None;
        }
        result
    }
}

impl PromiseReceiver {
    pub fn try_recv(&mut self) -> Result<Result<Value, String>, TryRecvError> {
        // Cooperative VM dispatch polls periodically. A temporary waiter avoids
        // replacing the wakers registered by other tasks on this same promise.
        let mut waiter = self.clone();
        let mut context = Context::from_waker(futures::task::noop_waker_ref());
        match Pin::new(&mut waiter).poll(&mut context) {
            Poll::Ready(Ok(value)) => Ok(value),
            Poll::Ready(Err(_)) => Err(TryRecvError::Closed),
            Poll::Pending => Err(TryRecvError::Empty),
        }
    }
}
