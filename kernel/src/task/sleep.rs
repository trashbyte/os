///////////////////////////////////////////////////////////////////////////////L
///////////////////////////////////////////////////////////////////////////////L

use crate::time::Instant;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use core::sync::atomic::{Ordering, AtomicBool};
use futures_util::task::AtomicWaker;
use crate::task::executor::GLOBAL_EXECUTOR;

#[derive(Debug)]
pub struct Sleeper {
    waker: Arc<AtomicWaker>,
    done: Arc<AtomicBool>,
}

impl Future for Sleeper {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        return if self.done.load(Ordering::Acquire) {
            Poll::Ready(())
        }
        else {
            self.waker.take();
            self.waker.register(cx.waker());
            Poll::Pending
        }
    }
}

pub fn sleep(duration: Duration) -> impl Future<Output = ()> {
    let waker = Arc::new(AtomicWaker::new());
    let done = Arc::new(AtomicBool::new(false));
    let expires = Instant::now() + duration;
    GLOBAL_EXECUTOR.get().unwrap().add_sleeper(waker.clone(), expires, done.clone());
    Sleeper { waker, done }
}
