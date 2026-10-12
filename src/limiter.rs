use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use governor::{
    DefaultDirectRateLimiter, Quota, RateLimiter,
    clock::{Clock, DefaultClock},
};
use uuid::Uuid;

// shortcut: per-process limiters; each process gets rate / WORKER_PROCESSES. Upgrade to a Postgres-backed limiter if we run many workers.
pub struct SendLimiter {
    /// (configured rate, limiter). governor cannot change a rate in place, so a new rate means a new limiter.
    global: Mutex<(u32, Arc<DefaultDirectRateLimiter>)>,
    accounts: Mutex<HashMap<Uuid, (u32, Arc<DefaultDirectRateLimiter>)>>,
    divisor: u32,
}

fn quota(rate: u32, divisor: u32) -> Quota {
    let per_proc = (rate / divisor.max(1)).max(1);
    Quota::per_second(NonZeroU32::new(per_proc).unwrap_or(NonZeroU32::MIN))
}

/// The SES rate, capped by our own ceiling. A bad answer (NaN, 0, -1 for "no limit") is ignored; never below 1.
pub fn effective_rate(ceiling: u32, ses: Option<f64>) -> u32 {
    match ses {
        Some(r) if r.is_finite() && r > 0.0 => ceiling.min(r.floor().max(1.0) as u32),
        _ => ceiling,
    }
}

impl SendLimiter {
    pub fn new(global_rate: u32, worker_processes: u32) -> Self {
        Self {
            global: Mutex::new((
                global_rate,
                Arc::new(RateLimiter::direct(quota(global_rate, worker_processes))),
            )),
            accounts: Mutex::new(HashMap::new()),
            divisor: worker_processes.max(1),
        }
    }

    pub fn global_rate(&self) -> u32 {
        self.global.lock().unwrap_or_else(|p| p.into_inner()).0
    }

    pub fn set_global_rate(&self, rate: u32) {
        let mut g = self.global.lock().unwrap_or_else(|p| p.into_inner());
        if g.0 != rate {
            let fresh = RateLimiter::direct(quota(rate, self.divisor));
            // Start empty: a fresh limiter's full burst would come on top of what the old one just sent.
            while fresh.check().is_ok() {}
            *g = (rate, Arc::new(fresh));
        }
    }

    fn global(&self) -> Arc<DefaultDirectRateLimiter> {
        self.global
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .1
            .clone()
    }

    fn account(&self, id: Uuid, rate: u32) -> Arc<DefaultDirectRateLimiter> {
        let mut map = self.accounts.lock().unwrap_or_else(|p| p.into_inner());
        match map.get(&id) {
            Some((r, l)) if *r == rate => l.clone(),
            _ => {
                let l = Arc::new(RateLimiter::direct(quota(rate, self.divisor)));
                map.insert(id, (rate, l.clone()));
                l
            }
        }
    }

    /// Account limit counts emails; the SES limit counts recipients (that is how SES counts).
    pub async fn wait(&self, account_id: Uuid, account_rate: u32, recipients: usize) {
        self.account(account_id, account_rate).until_ready().await;
        for _ in 0..recipients.max(1) {
            // Re-read the limiter on every try: after a rate change, waiters move to the new one.
            loop {
                match self.global().check() {
                    Ok(()) => break,
                    Err(later) => {
                        tokio::time::sleep(later.wait_time_from(DefaultClock::default().now()))
                            .await
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn the_rate_follows_ses_but_never_passes_the_ceiling() {
        assert_eq!(effective_rate(14, None), 14);
        assert_eq!(effective_rate(14, Some(2.0)), 2);
        assert_eq!(effective_rate(14, Some(500.0)), 14);
        assert_eq!(effective_rate(14, Some(0.5)), 1);
        assert_eq!(effective_rate(14, Some(0.0)), 14);
        assert_eq!(effective_rate(14, Some(-1.0)), 14);
        assert_eq!(effective_rate(14, Some(f64::NAN)), 14);
    }

    #[test]
    fn a_new_rate_swaps_the_limiter_and_the_same_rate_keeps_it() {
        let l = SendLimiter::new(14, 1);
        let before = l.global.lock().unwrap().1.clone();
        l.set_global_rate(14);
        assert!(Arc::ptr_eq(&before, &l.global.lock().unwrap().1));
        l.set_global_rate(2);
        assert_eq!(l.global_rate(), 2);
        assert!(!Arc::ptr_eq(&before, &l.global.lock().unwrap().1));
    }

    /// A fresh governor limiter starts with a full burst. After a rate change that burst would come on top
    /// of what the old limiter just sent, so the new one must start empty.
    #[tokio::test]
    async fn a_new_rate_starts_without_a_free_burst() {
        let l = SendLimiter::new(1000, 1);
        l.set_global_rate(5);
        let id = Uuid::now_v7();
        let start = Instant::now();
        l.wait(id, 1000, 5).await; // 5 recipients at 5/s, no burst: about 1 s
        assert!(
            start.elapsed().as_millis() >= 700,
            "took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn slows_down_past_the_rate() {
        let l = SendLimiter::new(1000, 1);
        let id = Uuid::now_v7();
        let start = Instant::now();
        for _ in 0..10 {
            l.wait(id, 5, 1).await; // 5/s with a burst of 5: 10 sends need about 1 s
        }
        assert!(
            start.elapsed().as_millis() >= 800,
            "took {:?}",
            start.elapsed()
        );
    }
}
