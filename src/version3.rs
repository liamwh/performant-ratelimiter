use chrono::{DateTime, Duration, Utc};
use crossbeam_queue::ArrayQueue;
use crossbeam_skiplist::SkipMap;
use std::net::IpAddr;

const MAX_REQUESTS: usize = 100;
const MAX_REQUESTS_DURATION_SECONDS: i64 = 60;

#[derive(Debug, Default)]
pub struct RateLimiter3 {
    requests: SkipMap<IpAddr, ArrayQueue<DateTime<Utc>>>,
}

impl RateLimiter3 {
    pub fn new() -> Self {
        RateLimiter3 {
            requests: SkipMap::new(),
        }
    }

    pub fn ratelimit3(&self, src_ip: IpAddr, timestamp: DateTime<Utc>) -> bool {
        let cutoff_time = timestamp - Duration::seconds(MAX_REQUESTS_DURATION_SECONDS);

        let entry = self
            .requests
            .get_or_insert_with(src_ip, || ArrayQueue::new(MAX_REQUESTS));
        let request_queue = entry.value();

        // Return early if the queue isn't full yet
        if !request_queue.is_full() {
            request_queue.push(timestamp).unwrap();
            return true;
        }

        let mut removed = 0;
        let mut valid_count = 0;
        while let Some(front_time) = request_queue.pop() {
            removed += 1;
            if front_time >= cutoff_time {
                request_queue.force_push(front_time);
                valid_count += 1;
            }
        }

        if removed > valid_count {
            request_queue.force_push(timestamp);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::{sync::Arc, thread};

    #[test]
    fn test_ratelimit3_under_max() {
        let rate_limiter = RateLimiter3::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit3(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit3_max_limit_still_permitted() {
        let rate_limiter = RateLimiter3::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit3(ip, now), true);
        }
    }

    #[test]
    fn test_ratelimit3_over_denied() {
        let rate_limiter = RateLimiter3::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS {
            assert_eq!(rate_limiter.ratelimit3(ip, now), true);
        }
        assert_eq!(rate_limiter.ratelimit3(ip, now), false);
    }

    #[test]
    fn test_ratelimit3_after_enough_time_allowed() {
        let rate_limiter = RateLimiter3::new();
        let ip = "127.0.0.1".parse::<IpAddr>().unwrap();
        let now = Utc::now();

        for _ in 0..MAX_REQUESTS - 1 {
            assert_eq!(rate_limiter.ratelimit3(ip, now), true);
        }

        let later = now + Duration::seconds(MAX_REQUESTS_DURATION_SECONDS + 1);
        assert_eq!(rate_limiter.ratelimit3(ip, later), true);
    }

    #[test]
    fn test_concurrent_ratelimit3() {
        const NUM_THREADS: usize = 10;
        let rate_limiter = Arc::new(RateLimiter3::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        (0..NUM_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    for _ in 0..MAX_REQUESTS - 1 {
                        rate_limiter.ratelimit3(ip, now);
                    }
                })
            })
            .for_each(|thread| {
                thread.join().expect("Thread failed");
            });

        let total_requests = {
            let x = match rate_limiter.requests.get(&ip) {
                Some(queue) => queue.value().len(),
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
    fn test_ratelimit3_request_overlimit() {
        const THREAD_REQUESTS: usize = 60;
        const TOTAL_THREADS: usize = 2;
        const EXPECTED_DENIALS: usize = (THREAD_REQUESTS * TOTAL_THREADS) - MAX_REQUESTS;
        let rate_limiter = Arc::new(RateLimiter3::new());
        let ip = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse IP");
        let now = Utc::now();

        let results: Vec<_> = (0..TOTAL_THREADS)
            .map(|_| {
                let rate_limiter = Arc::clone(&rate_limiter);
                thread::spawn(move || {
                    let mut denied = 0;
                    for _ in 0..THREAD_REQUESTS {
                        if !rate_limiter.ratelimit3(ip, now) {
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
