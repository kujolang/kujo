use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Manages a collection of threads.
///
/// A new thread is created every time all the existing threads are full.
/// Any idle thread will automatically die after a few seconds.
pub struct TaskPool {
    sharing: Arc<Sharing>,
}

struct Sharing {
    // list of the tasks to be done by worker threads
    todo: Mutex<VecDeque<Box<dyn FnMut() + Send>>>,

    // condvar that will be notified whenever a task is added to `todo`
    condvar: Condvar,

    // number of total worker threads running
    active_tasks: AtomicUsize,

    // number of idle worker threads
    waiting_tasks: AtomicUsize,
}

/// Minimum number of active threads.
static MIN_THREADS: usize = 4;

struct Registration<'a> {
    nb: &'a AtomicUsize,
}

impl<'a> Registration<'a> {
    fn new(nb: &'a AtomicUsize) -> Registration<'a> {
        nb.fetch_add(1, Ordering::Release);
        Registration { nb }
    }
}

impl<'a> Drop for Registration<'a> {
    fn drop(&mut self) {
        self.nb.fetch_sub(1, Ordering::Release);
    }
}

impl TaskPool {
    pub fn new() -> TaskPool {
        let pool = TaskPool {
            sharing: Arc::new(Sharing {
                todo: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active_tasks: AtomicUsize::new(0),
                waiting_tasks: AtomicUsize::new(0),
            }),
        };

        for _ in 0..MIN_THREADS {
            pool.add_thread(None)
        }

        pool
    }

    /// Executes a function in a thread.
    /// If no thread is available, spawns a new one.
    pub fn spawn(&self, code: Box<dyn FnMut() + Send>) {
        let mut queue = self.sharing.todo.lock().unwrap();

        // Queued tasks already reserve idle workers that have not yet woken.
        // Otherwise persistent connections can strand a newly queued connection.
        if self.sharing.waiting_tasks.load(Ordering::Acquire) <= queue.len() {
            self.add_thread(Some(code));
        } else {
            queue.push_back(code);
            self.sharing.condvar.notify_one();
        }
    }

    fn add_thread(&self, initial_fn: Option<Box<dyn FnMut() + Send>>) {
        let sharing = self.sharing.clone();

        thread::spawn(move || {
            let sharing = sharing;
            let _active_guard = Registration::new(&sharing.active_tasks);

            if let Some(mut f) = initial_fn {
                f();
            }

            loop {
                let mut task: Box<dyn FnMut() + Send> = {
                    let mut todo = sharing.todo.lock().unwrap();

                    let task;
                    loop {
                        if let Some(poped_task) = todo.pop_front() {
                            task = poped_task;
                            break;
                        }
                        let _waiting_guard = Registration::new(&sharing.waiting_tasks);

                        let received =
                            if sharing.active_tasks.load(Ordering::Acquire) <= MIN_THREADS {
                                todo = sharing.condvar.wait(todo).unwrap();
                                true
                            } else {
                                let (new_lock, waitres) = sharing
                                    .condvar
                                    .wait_timeout(todo, Duration::from_millis(5000))
                                    .unwrap();
                                todo = new_lock;
                                !waitres.timed_out()
                            };

                        if !received && todo.is_empty() {
                            return;
                        }
                    }

                    task
                };

                task();
            }
        });
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.sharing.active_tasks.store(999_999_999, Ordering::Release);
        self.sharing.condvar.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    #[test]
    fn queued_connections_reserve_idle_workers() {
        let pool = TaskPool::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while pool.sharing.waiting_tasks.load(Ordering::Acquire) < MIN_THREADS {
            assert!(Instant::now() < deadline, "idle workers did not start");
            thread::sleep(Duration::from_millis(1));
        }
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        {
            // Model the reachable window after enqueueing connections but before
            // notified workers reacquire the queue lock. All idle workers are
            // already reserved by queued, potentially long-lived connections.
            let mut queue = pool.sharing.todo.lock().unwrap();
            for _ in 0..MIN_THREADS {
                let release = release.clone();
                queue.push_back(Box::new(move || {
                    let (lock, wake) = &*release;
                    let mut done = lock.lock().unwrap();
                    while !*done {
                        done = wake.wait(done).unwrap();
                    }
                }));
            }
        }
        let (send, receive) = mpsc::channel();
        pool.spawn(Box::new(move || {
            send.send(()).unwrap();
        }));
        pool.sharing.condvar.notify_all();
        let progressed = receive.recv_timeout(Duration::from_secs(2)).is_ok();
        // Always release owned tasks, including when testing the broken code.
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        if !progressed {
            receive.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        assert!(progressed, "a new connection waited behind occupied workers");
    }
}
