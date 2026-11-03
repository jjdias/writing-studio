use std::time::{Duration, Instant};

/// Debounce policy for local autosave.
///
/// Unsaved work is flushed after `wait` has passed since the last edit.
/// There is no network step: callers write to the local project store.
pub fn autosave_due(
    has_unsaved: bool,
    last_edit: Option<Instant>,
    now: Instant,
    wait: Duration,
) -> bool {
    if !has_unsaved {
        return false;
    }
    match last_edit {
        None => true,
        Some(edited) => now.saturating_duration_since(edited) >= wait,
    }
}

pub fn autosave_wait(seconds: u32) -> Duration {
    Duration::from_secs(u64::from(seconds.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_until_idle_interval_elapses() {
        let start = Instant::now();
        let wait = Duration::from_secs(2);
        assert!(!autosave_due(true, Some(start), start, wait));
        assert!(autosave_due(
            true,
            Some(start),
            start + Duration::from_secs(2),
            wait
        ));
    }

    #[test]
    fn skips_when_nothing_is_dirty() {
        let now = Instant::now();
        assert!(!autosave_due(false, Some(now), now, Duration::from_secs(1)));
    }

    #[test]
    fn saves_dirty_work_without_a_timestamp() {
        assert!(autosave_due(true, None, Instant::now(), Duration::from_secs(5)));
    }
}
