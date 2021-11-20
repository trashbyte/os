///////////////////////////////////////////////////////////////////////////////L
///////////////////////////////////////////////////////////////////////////////L

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::{RefCell, RefMut};
use core::ops::{Deref, DerefMut};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_util::task::AtomicWaker;
use alloc::boxed::Box;
use crossbeam::queue::ArrayQueue;
use alloc::sync::Arc;

pub struct AsyncMutex<T> {
    locked: AtomicBool,
    inner: RefCell<T>,
    queue: Option<Box<ArrayQueue<Arc<AtomicWaker>>>>
}
impl<T> AsyncMutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            inner: RefCell::new(value),
            queue: None
        }
    }

    fn enqueue(&self, waker: Arc<AtomicWaker>) {
        if self.queue.is_none() {
            // shh it's fine
            unsafe {
                *(&self.queue as *const _ as *mut _)
                    = Some(Box::new(ArrayQueue::<Arc<AtomicWaker>>::new(20)));
            }
        }
        self.queue.as_ref().unwrap().push(waker)
            .expect("AsyncMutex waker queue is full");
    }

    pub fn lock(&self) -> AsyncMutexFuture<'_, T> {
        let waker = Arc::new(AtomicWaker::new());
        self.enqueue(waker.clone());
        AsyncMutexFuture { waker, mutex: &self }
    }

    pub fn try_lock(&self) -> Option<AsyncMutexLock<'_, '_, T>> {
        if self.locked.swap(true, Ordering::Acquire) {
            // returned true -> was already locked
            None
        }
        else {
            // was false, is now true -> we have the lock
            Some(AsyncMutexLock {
                inner: self.inner.borrow_mut(),
                mutex: &self
            })
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
        if let Some(waker) = self.queue.as_ref().unwrap().pop() {
            waker.wake();
        }
    }
}
unsafe impl<T> Sync for AsyncMutex<T> {}

pub struct AsyncMutexFuture<'a, T> {
    waker: Arc<AtomicWaker>,
    mutex: &'a AsyncMutex<T>,
}

impl<'a, T> Future for AsyncMutexFuture<'a, T> {
    type Output = AsyncMutexLock<'a, 'a, T>;

    // TODO: queueing
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.mutex.try_lock() {
            Some(lock) => Poll::Ready(lock),
            None => {
                self.waker.take();
                self.waker.register(cx.waker());
                Poll::Pending
            }
        }
    }
}

pub struct AsyncMutexLock<'a, 'b, T> {
    inner: RefMut<'b, T>,
    mutex: &'a AsyncMutex<T>
}
impl<T> Deref for AsyncMutexLock<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}
impl<T> DerefMut for AsyncMutexLock<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}
impl<T> Drop for AsyncMutexLock<'_, '_, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}
