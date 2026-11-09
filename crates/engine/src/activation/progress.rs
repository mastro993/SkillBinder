//! How far a change that writes and removes copies has got. The worker counts each pass's pairs
//! while a tracked job runs; the client reads the counts without queueing behind the worker, as it
//! reads a running scan.

use skillbinder_proto::{ActivationProgress, ActivationStep, AppError, AppResult, SkillId};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Default)]
struct Record {
    /// The counts of the change in flight or the last one, until cleared.
    counts: Option<ActivationProgress>,
    /// A tracked job is running, so passes count their pairs.
    tracking: bool,
}

/// The progress record shared by the worker and the client.
#[derive(Clone, Default)]
pub(crate) struct Progress(Arc<Mutex<Record>>);

/// A tracked job. Dropping it, however the job ends, stops counting and keeps the counts.
#[must_use]
pub(crate) struct Tracking(Progress);

impl Drop for Tracking {
    fn drop(&mut self) {
        if let Ok(mut record) = self.0.0.lock() {
            record.tracking = false;
            if let Some(counts) = record.counts.as_mut() {
                counts.step = None;
            }
        }
    }
}

impl Progress {
    fn lock(&self) -> AppResult<MutexGuard<'_, Record>> {
        self.0.lock().map_err(|_| AppError::storage())
    }

    /// Empties the record, so a change queued next reads nothing until it begins.
    pub(crate) fn clear(&self) {
        if let Ok(mut record) = self.lock() {
            *record = Record::default();
        }
    }

    /// Starts counting at 0 of 0 for a tracked job.
    pub(crate) fn begin(&self) -> Tracking {
        if let Ok(mut record) = self.lock() {
            *record = Record {
                counts: Some(ActivationProgress::default()),
                tracking: true,
            };
        }
        Tracking(self.clone())
    }

    /// Adds `pairs` a pass is about to bring in line to the total.
    pub(crate) fn planned(&self, pairs: usize) {
        self.count(|counts| counts.total += pairs);
    }

    /// Names the pair a pass is about to bring in line: `directory` and `slug`, `removing` when
    /// the skill is no longer active there.
    pub(crate) fn working(&self, directory: &str, slug: &str, removing: bool) {
        self.count(|counts| {
            counts.step = Some(ActivationStep {
                skill: SkillId(slug.to_owned()),
                directory: directory.to_owned(),
                removing,
            });
        });
    }

    /// Counts one pair brought in line, which needs the user when `attention` is true.
    pub(crate) fn advanced(&self, attention: bool) {
        self.count(|counts| {
            counts.done += 1;
            counts.attention += usize::from(attention);
        });
    }

    fn count(&self, change: impl FnOnce(&mut ActivationProgress)) {
        if let Ok(mut record) = self.lock()
            && record.tracking
            && let Some(counts) = record.counts.as_mut()
        {
            change(counts);
        }
    }

    /// The counts of the change in flight, or of the last one since the record was cleared.
    pub(crate) fn current(&self) -> AppResult<Option<ActivationProgress>> {
        Ok(self.lock()?.counts.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(done: usize, total: usize, attention: usize) -> Option<ActivationProgress> {
        Some(ActivationProgress {
            done,
            total,
            attention,
            step: None,
        })
    }

    #[test]
    fn reports_outside_a_tracked_job_are_ignored() -> AppResult<()> {
        let progress = Progress::default();
        progress.planned(3);
        progress.advanced(true);
        assert_eq!(progress.current()?, None);
        let tracking = progress.begin();
        drop(tracking);
        progress.planned(3);
        progress.advanced(false);
        assert_eq!(progress.current()?, counts(0, 0, 0));
        Ok(())
    }

    #[test]
    fn dropping_tracking_stops_counting_and_keeps_the_counts() -> AppResult<()> {
        let progress = Progress::default();
        let tracking = progress.begin();
        progress.planned(4);
        progress.advanced(false);
        progress.working("/home/.claude/skills", "review", true);
        progress.advanced(true);
        progress.planned(2);
        let step = progress.current()?.and_then(|counts| counts.step);
        assert_eq!(
            step.map(|step| (step.skill.0, step.removing)),
            Some(("review".to_owned(), true))
        );
        drop(tracking);
        progress.advanced(true);
        assert_eq!(progress.current()?, counts(2, 6, 1));
        Ok(())
    }

    #[test]
    fn clear_empties_the_record() -> AppResult<()> {
        let progress = Progress::default();
        let tracking = progress.begin();
        progress.planned(1);
        progress.clear();
        assert_eq!(progress.current()?, None);
        progress.advanced(false);
        assert_eq!(progress.current()?, None);
        drop(tracking);
        Ok(())
    }
}
