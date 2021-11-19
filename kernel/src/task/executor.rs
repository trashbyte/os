///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::{Waker, Context, Poll};
use crossbeam::queue::ArrayQueue;
use alloc::task::Wake;
use core::sync::atomic::{AtomicBool, Ordering};
use conquer_once::spin::OnceCell;
use spin::Mutex;
use core::future::Future;
use core::pin::Pin;
use futures_util::task::AtomicWaker;
use crate::util::DoubleArrayQueue;
use crate::time::Instant;

pub static GLOBAL_EXECUTOR: OnceCell<Executor> = OnceCell::uninit();
static RUNNING: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Copy)]
pub enum ExecutorError {
    /// woken task queue full (do you need to increase queue size?)
    TaskQueueFull
}

#[derive(Debug, Clone)]
pub struct Executor(Arc<ExecutorInner>);
impl Executor {
    pub fn init() -> Self {
        let me = Executor(Arc::new(ExecutorInner {
            tasks: Mutex::new(BTreeMap::new()),
            // fixed-size queue to avoid large allocations inside interrupt handlers
            task_queue: ArrayQueue::new(100),
            waker_cache: Mutex::new(BTreeMap::new()),
            pending_spawns: DoubleArrayQueue::new(100),
            sleepers: DoubleArrayQueue::new(100),
            check_sleepers: AtomicBool::new(true),
        }));
        GLOBAL_EXECUTOR.try_init_once(|| me.clone())
            .expect("Executor can only be initialized once.");
        me
    }

    pub fn run(&self, async_entry: Task) -> ! {
        if *RUNNING.lock() {
            panic!("Executor can only be run once");
        }
        *RUNNING.lock() = true;

        if let Err(_) = self.0.task_queue.push(async_entry.id) {
            panic!("task queue is full when initializing first task. this shouldn't be possible and indicates a bug in the executor");
        }
        if self.0.tasks.lock().insert(async_entry.id, async_entry).is_some() {
            panic!("task with same ID already in tasks map. this shouldn't be possible and indicates a bug in the executor");
        }

        self.0.run() // -> !
    }

    pub fn add_sleeper(&self, waker: Arc<AtomicWaker>, inst: Instant, done: Arc<AtomicBool>) {
        self.0.sleepers.get_alt().push((waker, inst, done))
            .expect("sleepers queue full");
    }

    fn wake_task(&self, id: TaskId) -> Result<(), TaskId> {
        self.0.task_queue.push(id)
    }

    pub fn spawn(&self, task: Task) -> ExecutorSpawnFuture {
        let waker = Arc::new(AtomicWaker::new());
        let done = Arc::new(AtomicBool::new(false));
        let future = ExecutorSpawnFuture { waker: waker.clone(), done: done.clone() };
        self.0.pending_spawns.get().push((task, waker, done)).expect("pending spawns queue full");
        future
    }

    // called from interrupt handler
    pub fn sleep_tick_set(&self) {
        self.0.check_sleepers.store(true, Ordering::Relaxed);
        // after return from interrupt, executor begins loop again and checks sleepers
    }
}
unsafe impl Send for Executor {}
unsafe impl Sync for Executor {}

#[derive(Debug)]
pub struct ExecutorInner {
    tasks: Mutex<BTreeMap<TaskId, Task>>,
    task_queue: ArrayQueue<TaskId>,
    waker_cache: Mutex<BTreeMap<TaskId, Waker>>,
    pending_spawns: DoubleArrayQueue<(Task, Arc<AtomicWaker>, Arc<AtomicBool>)>,
    sleepers: DoubleArrayQueue<(Arc<AtomicWaker>, Instant, Arc<AtomicBool>)>,
    check_sleepers: AtomicBool,
}

impl ExecutorInner {
    pub fn run(&self) -> ! {
        loop {
            if self.check_sleepers.load(Ordering::Relaxed) {
                self.check_sleepers();
            }
            self.run_ready_tasks();
            self.check_spawns();
            self.sleep_if_idle();
        }
    }

    fn check_sleepers(&self) {
        self.sleepers.swap();
        let now = Instant::now();
        while let Some((waker, expires, done)) = self.sleepers.get().pop() {
            let remaining = now.until(expires);
            //crate::serial_println!("remaining {}", remaining.as_millis());
            if remaining.as_millis() <= 0 {
                done.store(true, Ordering::Release);
                waker.wake();
            }
            else {
                self.sleepers.get_alt().push((waker, expires, done))
                    .expect("Sleeper alt queue full");
            }
        }
        self.check_sleepers.store(false, Ordering::Release);
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;

        // It's possible that an interrupt could fire between the empty check and the halt.
        // This would cause the cpu to halt even though there's a (new) task in the queue.
        // To prevent this, we disable interrupts during the check, and use the atomic
        // enable_and_hlt function to prevent the same race condition there.
        // We shouldn't need to worry about PENDING_SPAWNS since it should only be updated
        // by tasks executed by this executor, i.e. an update can't happen here.
        // TODO: when SMP is implemented, make sure this executor is locked to one core
        // or fix this implementation to be SMP-safe
        interrupts::disable();
        if self.task_queue.is_empty() && self.pending_spawns.get().is_empty() && self.sleepers.get().is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    fn check_spawns(&self) {
        while let Some((task, waker, done)) = self.pending_spawns.get().pop() {
            if let Err(_) = self.task_queue.push(task.id) {
                // queue full, delay spawn
                self.pending_spawns.get_alt().push((task, waker, done));
            }
            else {
                if self.tasks.lock().insert(task.id, task).is_some() {
                    panic!("task with same ID already in tasks map. this shouldn't be possible and indicates a bug in the executor");
                }
                done.fetch_xor(true, Ordering::Release);
                crate::serial_println!("task pushed. total tasks: {}", self.tasks.lock().len());
                waker.wake();
            }
        }
        self.pending_spawns.swap();
    }

    fn run_ready_tasks(&self) {
        // pop woken tasks from the queue
        while let Some(task_id) = self.task_queue.pop() {
            let mut tasks_lock = self.tasks.lock();
            let task = match tasks_lock.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            // reuse cached waker if it exists, else create new waker
            let mut waker_cache_lock = self.waker_cache.lock();
            let waker = waker_cache_lock
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id));

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks_lock.remove(&task_id);
                    waker_cache_lock.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }
}

pub struct ExecutorSpawnFuture {
    waker: Arc<AtomicWaker>,
    done: Arc<AtomicBool>,
}

impl Future for ExecutorSpawnFuture {
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

struct TaskWaker {
    task_id: TaskId
}

impl TaskWaker {
    fn new(task_id: TaskId) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id
        }))
    }

    fn wake_task(&self) {
        GLOBAL_EXECUTOR.get().expect("GLOBAL_EXECUTOR is not initialized")
            .wake_task(self.task_id)
            .expect("woken task queue full (do you need to increase queue size in task::executor?)");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
