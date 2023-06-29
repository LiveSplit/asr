use core::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    vec::Vec,
};

type TaskList<'fut> = RefCell<Vec<Pin<Box<dyn Future<Output = ()> + 'fut>>>>;

/// Manages a list of tasks that can be spawned and run concurrently to each
/// other.
///
/// # Example
///
/// ```no_run
/// # use asr::future::Tasks;
/// # async fn example() {
/// let tasks = Tasks::new();
///
/// tasks.spawn(async {
///     // do some work
/// });
///
/// tasks.spawn_recursive(|tasks| async move {
///     // do some work
///     tasks.spawn(async {
///         // do some work
///     });
/// });
///
/// tasks.run().await;
/// # }
/// ```
pub struct Tasks<'fut> {
    // This type is explicitly not clone to ensure that you don't create an Rc
    // cycle (Task list owns futures who own the task list and so on).
    tasks: Rc<TaskList<'fut>>,
}

impl<'fut> Tasks<'fut> {
    /// Creates a new list of tasks to execute.
    pub fn new() -> Self {
        Self {
            tasks: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Runs all tasks in the list to completion. While the tasks are running,
    /// further tasks can be spawned.
    pub fn run<'tasks>(&'tasks self) -> RunTasks<'fut, 'tasks> {
        RunTasks {
            tasks: Vec::new(),
            freshly_added: &self.tasks,
        }
    }

    /// Spawns a new task to be run concurrently to the other tasks.
    pub fn spawn(&self, f: impl Future<Output = ()> + 'fut) {
        self.tasks.borrow_mut().push(Box::pin(f));
    }

    /// Spawns a new task to be run concurrently to the other tasks. The
    /// provided closure is passed a [`TaskSpawner`] that can be used to spawn
    /// further tasks.
    pub fn spawn_recursive<F>(&self, f: impl FnOnce(TaskSpawner<'fut>) -> F)
    where
        F: Future<Output = ()> + 'fut,
    {
        self.spawn(f(self.spawner()));
    }

    /// Returns a [`TaskSpawner`] that can be used to spawn tasks.
    pub fn spawner(&self) -> TaskSpawner<'fut> {
        TaskSpawner {
            tasks: Rc::downgrade(&self.tasks),
        }
    }
}

impl<'fut> Default for Tasks<'fut> {
    fn default() -> Self {
        Self::new()
    }
}

/// A type that can be used to spawn tasks.
#[derive(Clone)]
pub struct TaskSpawner<'fut> {
    tasks: Weak<TaskList<'fut>>,
}

impl<'fut> TaskSpawner<'fut> {
    /// Spawns a new task to be run concurrently to the other tasks.
    pub fn spawn(&self, f: impl Future<Output = ()> + 'fut) {
        if let Some(tasks) = self.tasks.upgrade() {
            tasks.borrow_mut().push(Box::pin(f));
        }
    }

    /// Spawns a new task to be run concurrently to the other tasks. The
    /// provided closure is passed a [`TaskSpawner`] that can be used to spawn
    /// further tasks.
    pub fn spawn_recursive<F>(&self, f: impl FnOnce(Self) -> F)
    where
        F: Future<Output = ()> + 'fut,
    {
        self.spawn(f(self.clone()));
    }
}

/// A future that runs all tasks in the list to completion.
#[must_use = "You need to await this future."]
pub struct RunTasks<'fut, 'tasks> {
    tasks: Vec<Pin<Box<dyn Future<Output = ()> + 'fut>>>,
    freshly_added: &'tasks TaskList<'fut>,
}

impl Future for RunTasks<'_, '_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        this.tasks.extend(this.freshly_added.borrow_mut().drain(..));
        this.tasks.retain_mut(|f| f.as_mut().poll(cx).is_pending());

        if this.tasks.is_empty() && this.freshly_added.borrow().is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Wraps a future and allows it to spawn tasks that get run concurrently to the
/// wrapped future. The future completes once all tasks are complete.
///
/// Alternatively you can create a [`Tasks`] list for more control.
///
/// # Example
///
/// ```no_run
/// # use asr::future::run_tasks;
/// # async fn example() {
/// run_tasks(|tasks| async move {
///     // do some work
///
///     tasks.spawn(async {
///         // do some background work
///     });
///
///     // use spawn_recursive to spawn tasks that can spawn further tasks
///     tasks.spawn_recursive(|tasks| async move {
///         tasks.spawn(async {
///             // do some background work
///         });
///     });
///
///     // do some work
/// }).await;
/// # }
/// ```
pub async fn run_tasks<'fut, F>(f: impl FnOnce(TaskSpawner<'fut>) -> F)
where
    F: Future<Output = ()> + 'fut,
{
    let tasks = Tasks::new();
    tasks.spawn_recursive(f);
    tasks.run().await;
}
