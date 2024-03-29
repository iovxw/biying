use std::future::Future;
use std::pin::Pin;
use std::thread;

use futures::future::{pending, poll_fn};
use lazy_static::lazy_static;
use tokio::runtime::{self, Handle};

lazy_static! {
    static ref HANDLE: Handle = {
        let rt = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let handle = rt.handle().clone();
        thread::spawn(move || rt.block_on(pending::<()>()));
        handle
    };
}

pub async fn enter_tokio<T>(mut f: impl Future<Output = T>) -> T {
    poll_fn(|context| {
        let _guard = HANDLE.enter();
        // Safety: pinned on stack, and we are in an async fn
        // WARN: DO NOT use f in other places
        let f = unsafe { Pin::new_unchecked(&mut f) };
        f.poll(context)
    })
    .await
}
