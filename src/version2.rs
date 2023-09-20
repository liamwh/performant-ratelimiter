use super::*;
use chrono::{DateTime, Duration, Utc};
use crossbeam_skiplist::SkipMap;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct RateLimiter2 {
    requests: SkipMap<IpAddr, RwLock<VecDeque<DateTime<Utc>>>>,
}

impl RateLimiter2 {
    pub fn new() -> Self {
        RateLimiter2 {
            requests: SkipMap::new(),
        }
    }

    pub fn ratelimit2(&self, src_ip: IpAddr, timestamp: DateTime<Utc>) -> bool {
        let cutoff_time = timestamp - Duration::seconds(MAX_REQUESTS_DURATION_SECONDS);

        let request_queue = self
            .requests
            .get_or_insert_with(src_ip, || RwLock::new(VecDeque::new()));

        let mut locked_queue = request_queue.value().write().unwrap();

        while let Some(front_time) = locked_queue.front() {
            if *front_time < cutoff_time {
                locked_queue.pop_front();
            } else {
                break;
            }
        }

        if locked_queue.len() >= MAX_REQUESTS {
            return false;
        }

        locked_queue.push_back(timestamp);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{sync::Arc, thread};

    #[test]
    fn test_ratelimit2_under_max() {
        let rate_limiter = RateLimiter2::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit2(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit2_max_limit_still_permitted() {
        let rate_limiter = RateLimiter2::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit2(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit2_over_denied() {
        let rate_limiter = RateLimiter2::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit2(ip, now), true);
        }
        assert_eq!(rate_limiter.ratelimit2(ip, now), false);
    }

    #[test]
    fn test_ratelimit2_after_enough_time_allowed() {
        let rate_limiter = RateLimiter2::new();

        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit2(ip, now), true);
        }

        let later = now + Duration::seconds(MAX_REQUESTS_DURATION_SECONDS + 1);
        assert_eq!(rate_limiter.ratelimit2(ip, later), true);
    }

    #[test]
    fn test_concurrent_ratelimit2() {
        const NUM_THREADS: usize = 10;
        let rate_limiter = Arc::new(RwLock::new(RateLimiter2::new()));
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        (0..NUM_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    for _ in 0..MAX_REQUESTS - 1 {
                        let rl = rate_limiter.write().unwrap();
                        rl.ratelimit2(ip, now);
                    }
                })
            })
            .for_each(|thread| {
                thread.join().expect("Thread failed");
            });

        let total_requests = {
            let rl = rate_limiter.read().unwrap();
            let x = match rl.requests.get(&ip) {
                Some(queue) => queue.value().read().unwrap().len(),
                None => 0,
            };
            x
        };
        assert!(
            total_requests <= MAX_REQUESTS * NUM_THREADS,
            "Number of requests exceeded expected limit"
        );
    }

    #[test]
    fn test_ratelimit2_request_overlimit() {
        const THREAD_REQUESTS: usize = 60;
        const TOTAL_THREADS: usize = 2;
        const EXPECTED_DENIALS: usize = (THREAD_REQUESTS * TOTAL_THREADS) - MAX_REQUESTS;
        let rate_limiter = Arc::new(RwLock::new(RateLimiter2::new()));
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        let results: Vec<_> = (0..TOTAL_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    let mut denied = 0;
                    for _ in 0..THREAD_REQUESTS {
                        let rl = rate_limiter.write().unwrap();
                        if !rl.ratelimit2(ip, now) {
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
