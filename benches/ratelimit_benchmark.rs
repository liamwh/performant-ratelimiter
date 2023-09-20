use chrono::Utc;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ratelimit::{RateLimiter0, RateLimiter1, RateLimiter2, RateLimiter3};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

mod perf;

fn random_ip() -> IpAddr {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    IpAddr::V4(std::net::Ipv4Addr::new(
        rng.gen::<u8>(),
        rng.gen::<u8>(),
        rng.gen::<u8>(),
        rng.gen::<u8>(),
    ))
}

fn benchmark_ratelimiter0_tokio(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = Arc::new(RateLimiter0::new());

    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter0_tokio", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            let rate_limiter = Arc::clone(&rate_limiter);
            b.to_async(tokio::runtime::Builder::new_multi_thread().build().unwrap())
                .iter(|| async {
                    for chunk in random_ips.chunks(CHUNK_SIZE) {
                        let tasks: Vec<_> = chunk
                            .iter()
                            .map(|&ip| {
                                let rate_limiter = Arc::clone(&rate_limiter);
                                tokio::task::spawn(async move {
                                    rate_limiter.ratelimit0(ip, Utc::now());
                                })
                            })
                            .collect();

                        futures::future::try_join_all(tasks)
                            .await
                            .expect("One of the tasks failed.");
                    }
                });
        },
    );
}

fn benchmark_ratelimiter0(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = RateLimiter0::new();

    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter0", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.iter(|| {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    for &ip in chunk {
                        rate_limiter.ratelimit0(ip, Utc::now());
                    }
                }
            });
        },
    );

    group.finish();
}

fn benchmark_ratelimiter1_tokio(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = Arc::new(RateLimiter1::new());

    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter1_tokio", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.to_async(tokio::runtime::Builder::new_multi_thread().build().unwrap())
                .iter(|| async {
                    for chunk in random_ips.chunks(CHUNK_SIZE) {
                        let tasks: Vec<_> = chunk
                            .iter()
                            .map(|&ip| {
                                let rate_limiter = Arc::clone(&rate_limiter);
                                tokio::task::spawn(async move {
                                    rate_limiter.ratelimit1(ip, Utc::now());
                                })
                            })
                            .collect();

                        futures::future::try_join_all(tasks)
                            .await
                            .expect("One of the tasks failed.");
                    }
                });
        },
    );
}

fn benchmark_ratelimiter1(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = RateLimiter1::new();

    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter1", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.iter(|| {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    for &ip in chunk {
                        rate_limiter.ratelimit1(ip, Utc::now());
                    }
                }
            });
        },
    );

    group.finish();
}

fn benchmark_ratelimiter2_tokio(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = Arc::new(RateLimiter2::new());
    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();
    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter2_tokio", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.to_async(
                tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap(),
            )
            .iter(|| async {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    let tasks: Vec<_> = chunk
                        .iter()
                        .map(|&ip| {
                            let rate_limiter = Arc::clone(&rate_limiter);
                            tokio::task::spawn(async move {
                                rate_limiter.ratelimit2(ip, Utc::now());
                            })
                        })
                        .collect();

                    futures::future::try_join_all(tasks)
                        .await
                        .expect("One of the tasks failed.");
                }
            });
        },
    );

    group.finish();
}

fn benchmark_ratelimiter2(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = RateLimiter2::new();
    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter2", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.iter(|| {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    for &ip in chunk {
                        rate_limiter.ratelimit2(ip, Utc::now());
                    }
                }
            });
        },
    );

    group.finish();
}

fn benchmark_ratelimiter3_tokio(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = Arc::new(RateLimiter3::new());
    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();
    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter3_tokio", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.to_async(
                tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap(),
            )
            .iter(|| async {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    let tasks: Vec<_> = chunk
                        .iter()
                        .map(|&ip| {
                            let rate_limiter = Arc::clone(&rate_limiter);
                            tokio::task::spawn(async move {
                                rate_limiter.ratelimit3(ip, Utc::now());
                            })
                        })
                        .collect();

                    futures::future::try_join_all(tasks)
                        .await
                        .expect("One of the tasks failed.");
                }
            });
        },
    );

    group.finish();
}

fn benchmark_ratelimiter3(c: &mut Criterion) {
    const NUM_REQUESTS: usize = 1_000_000;
    const CHUNK_SIZE: usize = 1000;
    let rate_limiter = RateLimiter3::new();
    let random_ips: Vec<IpAddr> = (0..NUM_REQUESTS).map(|_| random_ip()).collect();

    let mut group = c.benchmark_group("ratelimiter_benchmarks");
    group.measurement_time(Duration::new(45, 0));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new("ratelimiter3", NUM_REQUESTS),
        &random_ips,
        |b, random_ips| {
            b.iter(|| {
                for chunk in random_ips.chunks(CHUNK_SIZE) {
                    for &ip in chunk {
                        rate_limiter.ratelimit3(ip, Utc::now());
                    }
                }
            });
        },
    );

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
    targets = benchmark_ratelimiter0_tokio, benchmark_ratelimiter1_tokio, benchmark_ratelimiter2_tokio, benchmark_ratelimiter3_tokio,
    benchmark_ratelimiter0, benchmark_ratelimiter1, benchmark_ratelimiter2, benchmark_ratelimiter3
}
criterion_main!(benches);
