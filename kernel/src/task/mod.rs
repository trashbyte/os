///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use core::{future::Future, pin::Pin};
use alloc::boxed::Box;
use core::task::{Poll, Context};
use core::sync::atomic::{AtomicU64, Ordering};
use core::fmt::{Debug, Formatter};

pub mod keyboard;
pub mod executor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context<'_>) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

impl Debug for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Task {{ id: {:?}, future: Pin<Box<dyn Future<Output = ()>>> }}", self.id)
    }
}