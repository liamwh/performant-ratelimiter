# Performant Ratelimiter

I wanted to play around with writing performant Rust code, so I decided to try implementing a dummy ratelimiting middleware.

### TL;DR

Checkout the [results.](https://github.com/liamwh/performant-ratelimiter#results)

## Version comparisons

### [RateLimiter Version 0](https://github.com/liamwh/performant-ratelimiter/blob/main/src/version0.rs) - RwLock HashMap with VecDeque values

The first iteration of `RateLimiter` uses the following structure.

```rs
pub struct RateLimiter0 {
    requests: RwLock<HashMap<IpAddr, VecDeque<DateTime<Utc>>>>,
}
```

Key Characteristics:

- Only uses the standard library, no external crates for data structures.
- **Ratelimit0 Method**: The `ratelimit0` function implements a rate-limiting mechanism based on a given source IP and timestamp. It first computes a `cutoff_time` to determine the relevancy of requests. Upon acquiring a write lock on the shared `requests` map, it retrieves (or initializes if non-existent) a queue of timestamps associated with the source IP. It then iterates through this queue, removing any timestamps older than the `cutoff_time`. If the length of the filtered queue surpasses a predefined maximum (i.e., `MAX_REQUESTS`), the function returns `false`, indicating that the rate limit has been exceeded; otherwise, it adds the new timestamp to the queue and returns `true`. This method is designed to be thread-safe by ensuring mutual exclusion using an `RwLock` around the entire `HashMap`.

### [RateLimiter Version 1](https://github.com/liamwh/performant-ratelimiter/blob/main/src/version1.rs) - No locks, SkipMap with VecDeque values

This iteration of `RateLimiter` uses the following structure:

```rs
pub struct RateLimiter1 {
    requests: SkipMap<IpAddr, VecDeque<DateTime<Utc>>>,
}
```

Key Characteristics:

- **Data Structure**: It stores the requests directly as `VecDeque<DateTime<Utc>>` without any locking mechanism.

- **Ratelimit1 Method**: The method ratelimit operates on this simple structure. It fetches the current request queue (or initializes a new one if none exists), then trims old requests, checks if the current request is within rate limits, and updates the map with the new request queue. However, this method is not thread-safe and thus the VecDeque is prone to race conditioncs.

> Note that race conditions do not violate Rust’s memory safety rules. A race between multiple threads can never cause memory errors or segfaults. A race condition is a logic error in its entirety.

### [RateLimiter Version 2](https://github.com/liamwh/performant-ratelimiter/blob/main/src/version2.rs) - SkipMap with RwLock'd VecDeques

The second version of `RateLimiter` introduces some modifications:

```rs
pub struct RateLimiter2 {
    requests: SkipMap<IpAddr, RwLock<VecDeque<DateTime<Utc>>>>,
}
```

Key Characteristics:

- **Data Structure**: It introduces an `RwLock` to make the data structure thread-safe, eliminating race conditions.

- **Ratelimit2 Method**: It fetches or initializes the request queue and then, within a write lock, trims old requests and checks the current request against the rate limits.

### [RateLimiter Version 3](https://github.com/liamwh/performant-ratelimiter/blob/main/src/version3.rs) - SkipMap with ArrayQueue values

```rs
pub struct RateLimiter3 {
    requests: SkipMap<IpAddr, ArrayQueue<DateTime<Utc>>>,
}
```

Key Characteristics:

- **Data Structure**: It uses an `ArrayQueue` instead of a `VecDeque` as a thread-safe data structure, eliminating race conditions.

## Benchmarks

I used [criterion](https://github.com/bheisler/criterion.rs) for benchmarking the performance, and [pprof](https://docs.rs/pprof/latest/pprof/) + [flamegraph](https://github.com/flamegraph-rs/flamegraph) for profiling the benches.

### Methodology

- **Random IP Generation**: Simulates diverse source IP addresses using a `random_ip()` function that produces random IPv4 addresses.
- **Requests**: Each test simulates **a million** requests.
- **Chunking**: Requests are processed in chunks of **1,000** at a time, leveraging the benefits of parallel processing.
- **Concurrency**: Utilizes the `tokio::runtime::Builder::new_multi_thread()` to process requests concurrently, maximizing the utilization of available CPU cores. This done to simulate an actual web server.
- **Measurement Duration**: Each benchmark iteration runs for a duration of **45 seconds** to ensure that enough samples are collected.
- **Repetitions**: Benchmarks are repeated **10 times** to account for variations and to ensure consistent results.
- **Error Handling**: The code expects all tasks to complete successfully. If any of the tasks fail, the benchmark will terminate with an error.

### Hardware employed

These benchmarks were ran on my local machine with the following specs:

| Property | Value                             |
| -------- | --------------------------------- |
| Kernel   | 5.15.90.1-microsoft-standard-WSL2 |
| CPU      | AMD Ryzen 7 3800X (4) @ 3.900GHz  |
| Memory   | 15998MiB                          |

### Results

The results can be found [here.](https://liamwh.github.io/performant-ratelimiter/)

#### Conclusion

Version 0 was the fastest, due to the `HashMap`'s underlying hashtable data structure which has a time complexity of O(1) for the average case for most operations (see table below). The contention for the `RwLock` was not an issue in this case. Should the amount of time per request increase, then the contention for the `RwLock` would become more of an issue, and the `SkipMap` would likely become the better choice.

| Operation/Characteristic | `std::collections::HashMap`   | `crossbeam::SkipMap`   |
| ------------------------ | ----------------------------- | ---------------------- |
| Underlying Structure     | Hash table                    | Skip list              |
| Insertion                | O(1) avg, O(n) worst          | O(log n) avg           |
| Deletion                 | O(1) avg, O(n) worst          | O(log n) avg           |
| Lookup                   | O(1) avg, O(n) worst          | O(log n) avg           |
| Iterating                | O(n) (arbitrary order)        | O(n) (ascending order) |
| Concurrency              | No (external synchronization) | Yes (lock-free)        |

#### Running the benchmarks

To run the benchmarks yourself, clone the repo and run:

`cargo bench --bench ratelimit_benchmark -- --profile-time=45`

- The index.html file for the benchmarks will be created at `target/criterion/report/index.html`
- The flamegraph of the benchmark will be created at `target/criterion/<name-of-benchmark>/profile/flamegraph.svg`
