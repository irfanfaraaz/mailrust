use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{Arc, Mutex},
};

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use uuid::Uuid;

// shortcut: per-process limiters; each process gets rate / WORKER_PROCESSES. Upgrade to a Postgres-backed limiter if we run many workers.
pub struct SendLimiter {
    global: DefaultDirectRateLimiter,
    accounts: Mutex<HashMap<Uuid, (u32, Arc<DefaultDirectRateLimiter>)>>,
    divisor: u32,
}

fn quota(rate: u32, divisor: u32) -> Quota {
    let per_proc = (rate / divisor.max(1)).max(1);
    Quota::per_second(NonZeroU32::new(per_proc).unwrap_or(NonZeroU32::MIN))
}

impl SendLimiter {
    pub fn new(global_rate: u32, worker_processes: u32) -> Self {
        Self {
            global: RateLimiter::direct(quota(global_rate, worker_processes)),
            accounts: Mutex::new(HashMap::new()),
            divisor: worker_processes.max(1),
        }
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
            self.global.until_ready().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

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
