use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc};
use core::task::{Waker, Context, Poll};
use crossbeam::queue::ArrayQueue;
use alloc::task::Wake;

#[derive(Debug)]
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            // fixed-size queue to avoid allocations inside interrupt handlers
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks map");
        }
        self.task_queue.push(task_id)
            .expect("woken task queue full (do you need to increase queue size in task::executor?)");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts;

        // It's possible that an interrupt could fire between the empty check and the halt.
        // This would cause the cpu to halt even though there's a (new) task in the queue.
        // To prevent this, we disable interrupts during the check, and use the atomic
        // enable_and_hlt function to prevent the same race condition there.
        interrupts::disable();
        if self.task_queue.is_empty() {
            interrupts::enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    fn run_ready_tasks(&mut self) {
        // destructure `self` to avoid borrow checker errors.
        // update when [https://github.com/rust-lang/rfcs/pull/2229](RFC2229) is implemented
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        // pop woken tasks from the queue
        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            // reuse cached waker if it exists, else create new waker
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // task done -> remove it and its cached waker
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id)
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
