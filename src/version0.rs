use super::*;
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct RateLimiter0 {
    requests: RwLock<HashMap<IpAddr, VecDeque<DateTime<Utc>>>>,
}

impl RateLimiter0 {
    pub fn new() -> Self {
        RateLimiter0 {
            requests: RwLock::new(HashMap::new()),
        }
    }

    pub fn ratelimit0(&self, src_ip: IpAddr, timestamp: DateTime<Utc>) -> bool {
        let cutoff_time = timestamp - Duration::seconds(MAX_REQUESTS_DURATION_SECONDS);

        let mut requests = self.requests.write().unwrap(); // In production code we'd handle
                                                           // the case of a poisoned lock
        let current_requests = requests.entry(src_ip).or_default();

        while let Some(front_time) = current_requests.front() {
            if *front_time < cutoff_time {
                current_requests.pop_front();
            } else {
                break;
            }
        }

        if current_requests.len() >= MAX_REQUESTS {
            return false;
        }

        current_requests.push_back(timestamp);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        thread,
    };

    #[test]
    fn test_ratelimit0_under_max() {
        let rate_limiter = RateLimiter0::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit0(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit0_max_limit_still_permitted() {
        let rate_limiter = RateLimiter0::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit0(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit0_over_denied() {
        let rate_limiter = RateLimiter0::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit0(ip, now), true);
        }
        assert_eq!(rate_limiter.ratelimit0(ip, now), false);
    }

    #[test]
    fn test_ratelimit0_after_enough_time_allowed() {
        let rate_limiter = RateLimiter0::new();

        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit0(ip, now), true);
        }

        let later = now + Duration::seconds(MAX_REQUESTS_DURATION_SECONDS + 1);
        assert_eq!(rate_limiter.ratelimit0(ip, later), true);
    }

    #[test]
    fn test_ratelimit0_concurrent_access_respects_max_requests_limit() {
        const NUM_THREADS: usize = 10;
        let rate_limiter = Arc::new(RateLimiter0::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();
        let total_requests: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        (0..NUM_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                let total_requests = Arc::clone(&total_requests);
                thread::spawn(move || {
                    for _ in 0..MAX_REQUESTS + 1 {
                        if rate_limiter.ratelimit0(ip, now) {
                            total_requests.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                })
            })
            .for_each(|thread| {
                thread.join().expect("Thread failed");
            });

        assert_eq!(total_requests.load(Ordering::SeqCst), MAX_REQUESTS);
    }

    #[test]
    fn test_ratelimiter0_request_overlimit() {
        const THREAD_REQUESTS: usize = 60;
        const TOTAL_THREADS: usize = 2;
        const EXPECTED_DENIALS: usize = (THREAD_REQUESTS * TOTAL_THREADS) - MAX_REQUESTS;
        let rate_limiter = Arc::new(RateLimiter0::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        let results: Vec<_> = (0..TOTAL_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    let mut denied = 0;
                    for _ in 0..THREAD_REQUESTS {
                        if !rate_limiter.ratelimit0(ip, now) {
                            denied += 1;
                        }
                    }
                    denied
                })
            })
            .map(|thread| thread.join().expect("Thread failed"))
            .collect();

        let total_denials: usize = results.iter().sum();
        assert!(
            total_denials >= EXPECTED_DENIALS,
            "Expected at least {} denials, but got {}",
            EXPECTED_DENIALS,
            total_denials
        );
    }
}
