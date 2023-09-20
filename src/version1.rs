use super::*;
use chrono::{DateTime, Duration, Utc};
use crossbeam_skiplist::SkipMap;
use std::collections::VecDeque;
use std::net::IpAddr;

#[derive(Debug, Default)]
pub struct RateLimiter1 {
    requests: SkipMap<IpAddr, VecDeque<DateTime<Utc>>>,
}

impl RateLimiter1 {
    pub fn new() -> Self {
        RateLimiter1 {
            requests: SkipMap::new(),
        }
    }

    pub fn ratelimit1(&self, src_ip: IpAddr, timestamp: DateTime<Utc>) -> bool {
        let mut current_requests = self
            .requests
            .get(&src_ip)
            .map(|r| r.value().clone())
            .unwrap_or_default();

        let cutoff_time = timestamp - Duration::seconds(MAX_REQUESTS_DURATION_SECONDS);
        while let Some(front_time) = current_requests.front() {
            if *front_time < cutoff_time {
                current_requests.pop_front();
            } else {
                break;
            }
        }

        if current_requests.len() >= MAX_REQUESTS {
            self.requests.insert(src_ip, current_requests);
            return false;
        }

        current_requests.push_back(timestamp);
        self.requests.insert(src_ip, current_requests);
        true
    }

    #[cfg(test)]
    pub fn requests(&self) -> &SkipMap<IpAddr, VecDeque<DateTime<Utc>>> {
        &self.requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{sync::Arc, thread};

    #[test]
    fn test_ratelimit1_under_max() {
        let rate_limiter = RateLimiter1::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit1(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit1_max_limit_still_permitted() {
        let rate_limiter = RateLimiter1::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit1(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit1_over_denied() {
        let rate_limiter = RateLimiter1::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit1(ip, now), true);
        }
        assert_eq!(rate_limiter.ratelimit1(ip, now), false);
    }

    #[test]
    fn test_ratelimit1_after_enough_time_allowed() {
        let rate_limiter = RateLimiter1::new();

        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit1(ip, now), true);
        }

        let later = now + Duration::seconds(MAX_REQUESTS_DURATION_SECONDS + 1);
        assert_eq!(rate_limiter.ratelimit1(ip, later), true);
    }

    #[test]
    fn test_ratelimit1_concurrent_ratelimit() {
        const NUM_THREADS: usize = 10;
        let rate_limiter = Arc::new(RateLimiter1::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        (0..NUM_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    for _ in 0..MAX_REQUESTS - 1 {
                        rate_limiter.ratelimit1(ip, now);
                    }
                })
            })
            .for_each(|thread| {
                thread.join().expect("Thread failed");
            });

        let total_requests = rate_limiter
            .requests()
            .get(&ip)
            .map(|r| r.value().len())
            .unwrap_or(0);
        assert!(
            total_requests <= MAX_REQUESTS * NUM_THREADS,
            "Number of requests exceeded expected limit"
        );
    }

    #[test]
    fn test_ratelimiter1_request_overlimit() {
        const THREAD_REQUESTS: usize = 60;
        const TOTAL_THREADS: usize = 2;
        const EXPECTED_DENIALS: usize = (THREAD_REQUESTS * TOTAL_THREADS) - MAX_REQUESTS;
        let rate_limiter = Arc::new(RateLimiter1::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        let results: Vec<_> = (0..TOTAL_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    let mut denied = 0;
                    for _ in 0..THREAD_REQUESTS {
                        if !rate_limiter.ratelimit1(ip, now) {
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
