use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

/// Shared work queue for thread pool
pub struct WorkQueue {
    queue: VecDeque<PathBuf>,
    active_tasks: usize,
    shutdown: bool,
}

/// Thread-safe work queue wrapper
pub struct SharedWorkQueue {
    inner: Arc<(Mutex<WorkQueue>, Condvar)>,
}

impl SharedWorkQueue {
    /// Create a new shared work queue
    pub fn new() -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(WorkQueue {
                    queue: VecDeque::new(),
                    active_tasks: 0,
                    shutdown: false,
                }),
                Condvar::new(),
            )),
        }
    }

    /// Add initial work item
    pub fn push_initial(&self, path: PathBuf) {
        let (lock, _) = &*self.inner;
        let mut queue = lock.lock().unwrap();
        queue.queue.push_back(path);
        queue.active_tasks = 1;
    }

    /// Try to get the next work item
    pub fn pop(&self) -> Option<PathBuf> {
        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock().unwrap();

        loop {
            // Check for shutdown FIRST - don't process any more items
            if queue.shutdown {
                return None;
            }

            // Try to get a task
            if let Some(path) = queue.queue.pop_front() {
                return Some(path);
            }

            // No tasks available
            if queue.active_tasks == 0 {
                // No tasks and no active workers, we're done
                queue.shutdown = true;
                cvar.notify_all();
                return None;
            }

            // Wait for new tasks
            queue = cvar.wait(queue).unwrap();
        }
    }

    /// Add multiple work items
    pub fn extend(&self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock().unwrap();
        
        // Don't add new work if we're shutting down
        if queue.shutdown {
            return;
        }

        let count = paths.len();
        
        // Add new tasks
        queue.queue.extend(paths);
        
        // Update active task count
        queue.active_tasks += count;
        
        // Wake up waiting workers
        cvar.notify_all();
    }

    /// Mark a task as complete
    pub fn complete_task(&self) {
        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock().unwrap();
        
        queue.active_tasks = queue.active_tasks.saturating_sub(1);
        
        // Wake up threads that might be waiting for completion
        cvar.notify_all();
    }

    /// Signal shutdown to all workers
    pub fn shutdown(&self) {
        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock().unwrap();
        queue.shutdown = true;
        cvar.notify_all();
    }

    /// Check if shutdown has been signaled
    pub fn is_shutdown(&self) -> bool {
        let (lock, _) = &*self.inner;
        let queue = lock.lock().unwrap();
        queue.shutdown
    }

    /// Clone the Arc for sharing between threads
    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Get optimal number of worker threads
pub fn get_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8)
        .min(crate::config::Config::MAX_THREADS)
}