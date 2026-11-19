//! The pool that runs Git network work off the worker, where a slow clone would hold up every
//! other command (design T4). At most three jobs run at once; later ones wait their turn, first
//! come first served. Threads start with work and end when none is left.

use skillbinder_proto::{AppError, AppResult};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::oneshot;

/// How many jobs run at once.
const THREADS: usize = 3;

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// Threads started and not yet ended.
    threads: usize,
}

/// The shared pool; clones run jobs on the same threads.
#[derive(Clone, Default)]
pub(crate) struct GitPool {
    queue: Arc<Mutex<Queue>>,
}

impl GitPool {
    fn lock(&self) -> MutexGuard<'_, Queue> {
        // A job runs outside the lock, so a panicking one never poisons it.
        self.queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Runs `work` once a thread is free. The receiver answers with its result, or errs when
    /// the job panicked.
    pub(crate) fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> AppResult<oneshot::Receiver<T>> {
        let (done, result) = oneshot::channel();
        let mut queue = self.lock();
        queue.jobs.push_back(Box::new(move || {
            let _ = done.send(work());
        }));
        if queue.threads < THREADS {
            queue.threads += 1;
            let pool = self.clone();
            let spawned = std::thread::Builder::new()
                .name("skillbinder-git".into())
                .spawn(move || pool.work());
            if spawned.is_err() {
                queue.threads -= 1;
                // Without any thread, nothing would ever run the job.
                if queue.threads == 0 {
                    queue.jobs.pop_back();
                    return Err(AppError::storage());
                }
            }
        }
        Ok(result)
    }

    /// Runs queued jobs until none is left, then ends the thread.
    fn work(&self) {
        let mut ending = Ending(Some(self.clone()));
        loop {
            let job = {
                let mut queue = self.lock();
                match queue.jobs.pop_front() {
                    Some(job) => job,
                    None => {
                        // Counted down under the lock that found the queue empty, so a job
                        // queued just after starts a thread of its own.
                        queue.threads -= 1;
                        ending.0 = None;
                        return;
                    }
                }
            };
            job();
        }
    }

    /// Waits until every queued and running job has ended. Jobs consult the shutdown signal, so
    /// a quit ends them promptly.
    pub(crate) fn wait_for_shutdown(&self) {
        while self.lock().threads > 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Counts a thread down when its job panics and unwinds it.
struct Ending(Option<GitPool>);

impl Drop for Ending {
    fn drop(&mut self) {
        if let Some(pool) = self.0.take() {
            let mut queue = pool.lock();
            queue.threads -= 1;
            // A job queued meanwhile, with every other thread busy, still gets a thread.
            if !queue.jobs.is_empty() && queue.threads == 0 {
                queue.threads += 1;
                let next = pool.clone();
                if std::thread::Builder::new()
                    .name("skillbinder-git".into())
                    .spawn(move || next.work())
                    .is_err()
                {
                    queue.threads -= 1;
                    queue.jobs.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn three_jobs_run_at_once_and_a_fourth_waits_its_turn() {
        let pool = GitPool::default();
        let (started, starts) = mpsc::channel();
        let mut releases = Vec::new();
        let mut results = Vec::new();
        for job in 0..4 {
            let (release, wait) = mpsc::channel::<()>();
            let started = started.clone();
            releases.push(release);
            results.push(
                pool.run(move || {
                    started.send(job).unwrap();
                    wait.recv().unwrap();
                    job
                })
                .unwrap(),
            );
        }
        let mut first: Vec<_> = (0..3)
            .map(|_| starts.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect();
        first.sort_unstable();
        assert_eq!(first, [0, 1, 2]);
        // The fourth waits for a free thread, however long the first three take.
        assert!(starts.recv_timeout(Duration::from_millis(200)).is_err());
        releases[1].send(()).unwrap();
        assert_eq!(starts.recv_timeout(Duration::from_secs(5)).unwrap(), 3);
        for release in [&releases[0], &releases[2], &releases[3]] {
            release.send(()).unwrap();
        }
        let answers: Vec<_> = results
            .into_iter()
            .map(|result| result.blocking_recv().unwrap())
            .collect();
        assert_eq!(answers, [0, 1, 2, 3]);
        pool.wait_for_shutdown();
        assert_eq!(pool.lock().threads, 0);
    }

    #[test]
    fn a_panicking_job_frees_its_thread() {
        let pool = GitPool::default();
        let panicked = pool.run(|| panic!("job failed")).unwrap();
        assert!(panicked.blocking_recv().is_err());
        assert_eq!(pool.run(|| 7).unwrap().blocking_recv().unwrap(), 7);
        pool.wait_for_shutdown();
    }
}
