use std::future::Future;
use std::pin::Pin;
use std::thread;

use futures::future::{pending, poll_fn};
use lazy_static::lazy_static;
use tokio::runtime::{Handle, Runtime};

lazy_static! {
    static ref HANDLE: Handle = {
        let mut rt = Runtime::new().unwrap();
        let handle = rt.handle().clone();
        thread::spawn(move || rt.block_on(pending::<()>()));
        handle
    };
}

pub async fn enter_tokio_<T>(mut f: impl Future<Output = T>) -> T {
    poll_fn(|context| {
        HANDLE.enter(|| {
            // Safety: pinned on stack, and we are in an async fn
            // WARN: DO NOT use f in other places
            let f = unsafe { Pin::new_unchecked(&mut f) };
            f.poll(context)
        })
    })
    .await
}

pub fn enter_tokio<T>(f: impl Future<Output = T>) -> impl Future<Output = T> {
    FixPoll(Some(async { enter_tokio_(f).await }))
}

// this is a workaround
// fix qmetaobject::execute_async
pub struct FixPoll<F>(Option<F>);

impl<F, T> Future for FixPoll<F>
where
    F: Future<Output = T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context) -> std::task::Poll<T> {
        let this = unsafe { &mut self.get_unchecked_mut().0 };
        if let Some(f) = this {
            let r = unsafe { Pin::new_unchecked(f) }.poll(cx);
            if r.is_ready() {
                this.take(); // drop
            }
            r
        } else {
            dbg!("nooo");
            std::task::Poll::Pending
        }
    }
}

impl<F> Drop for FixPoll<F> {
    fn drop(&mut self) {
        dbg!("DROP");
    }
}
