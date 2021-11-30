// ///////////////////////////////////////////////////////////////////////////////L
// ///////////////////////////////////////////////////////////////////////////////L

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use crossbeam::queue::ArrayQueue;
use futures_util::task::AtomicWaker;
use lock_api::{RawMutex, Mutex, GuardSend};

type Waker = Arc<AtomicWaker>;
type MaybeQueue = Option<Box<ArrayQueue<Waker>>>;

pub struct RawSpinMutex {
     pub locked: AtomicBool
}

impl RawSpinMutex {
    pub const fn new() -> Self { Self { locked: AtomicBool::new(false) } }
}

unsafe impl RawMutex for RawSpinMutex {
    const INIT: Self = Self { locked: AtomicBool::new(false) };

    type GuardMarker = GuardSend;

    fn lock(&self) {
        //let waker = Arc::new(AtomicWaker::new());
        //self.enqueue(waker.clone());
        //AsyncMutexFuture { waker, mutex: &self }
        while !self.try_lock() {}
    }

    fn try_lock(&self) -> bool {
        self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }

    unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
        // if let Some(waker) = self.queue.borrow_mut().as_mut().unwrap().pop() {
        //     waker.wake();
        // }
    }
}

// pub struct AsyncMutexFuture<'a, T> {
//     waker: Waker,
//     mutex: &'a AsyncMutex<T>,
// }
//
// impl<'a, T> Future for AsyncMutexFuture<'a, T> {
//     type Output = AsyncMutexLock<'a, 'a, T>;
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         match self.mutex.try_lock() {
//             Some(lock) => Poll::Ready(lock),
//             None => {
//                 self.waker.take();
//                 self.waker.register(cx.waker());
//                 Poll::Pending
//             }
//         }
//     }
// }

pub type SpinMutex<T> = lock_api::Mutex<RawSpinMutex, T>;
pub type SpinMutexGuard<'a, T> = lock_api::MutexGuard<'a, RawSpinMutex, T>;
