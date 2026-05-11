//! Criterion bench for HTTP request fan-out cost. Stream
//! `mission-criterion-load-tests`.
//!
//! Why this matters: every gateway request pays a fixed TCP +
//! parse + response-format overhead. We measure that baseline so
//! a regression that pessimizes the per-request loop shows up
//! before it hits production traffic.
//!
//! Brief drift note: the brief asked for booting the real argo
//! gateway and hitting `/health` + `/providers`. argo-cli is a
//! binary-only crate (no `lib.rs`) and its gateway routes are
//! `pub(crate)`, so a bench in `argo-cli/benches/` cannot call
//! them directly. We approximate with a minimal in-process TCP
//! listener that serves the same fixed-shape responses the real
//! gateway uses for `/health` and `/providers`. The bench
//! exercises tokio's accept-loop, the reqwest client pool, and
//! the connection-handling cost — the same dimensions a regression
//! in the gateway's outer loop would surface in.
//!
//! Two cases under one group:
//!   * `serial` — one reqwest client, 100 requests back-to-back.
//!   * `parallel_16` — 16 reqwest clients fanning out 1600 total
//!     requests via `JoinSet`. Exercises tokio's runtime + the
//!     listener's accept-loop concurrency.
//!
//! Run locally:
//!   `cargo bench -p argo-cli --bench gateway_request`
//!
//! NOT wired into CI (compile-only via `cargo build --benches`).
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::fmt::Display;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio::task::{JoinHandle, JoinSet};

const SERIAL_REQUESTS: usize = 100;
const PARALLEL_CLIENTS: usize = 16;
const PARALLEL_REQUESTS_PER_CLIENT: usize = 25;

const HEALTH_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: keep-alive\r\n\r\n{\"status\":\"ok\"}";
const PROVIDERS_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 16\r\nConnection: keep-alive\r\n\r\n{\"providers\":[]}";
const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

struct BenchServer {
    url: String,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl Drop for BenchServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.join.take() {
            j.abort();
        }
    }
}

fn gateway_request_rps(c: &mut Criterion) {
    let runtime = must(Runtime::new(), "tokio runtime");
    let server = must(runtime.block_on(spawn_server()), "spawn server");
    let server = Arc::new(server);
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(PARALLEL_CLIENTS)
        .build()
        .unwrap_or_else(|e| panic!("reqwest client: {e}"));

    let mut group = c.benchmark_group("gateway_request");
    group.sample_size(15);
    group.throughput(Throughput::Elements(SERIAL_REQUESTS as u64));

    let url_health = format!("{}/health", server.url);
    let url_providers = format!("{}/providers", server.url);

    group.bench_function("serial", |b| {
        b.iter(|| {
            must(
                runtime.block_on(serial_requests(&client, &url_health, SERIAL_REQUESTS)),
                "serial requests",
            );
        });
    });

    let parallel_total = PARALLEL_CLIENTS * PARALLEL_REQUESTS_PER_CLIENT;
    group.throughput(Throughput::Elements(parallel_total as u64));
    group.bench_function("parallel_16", |b| {
        b.iter_batched(
            || (client.clone(), url_providers.clone()),
            |(c, url)| {
                must(
                    runtime.block_on(parallel_requests(
                        c,
                        url,
                        PARALLEL_CLIENTS,
                        PARALLEL_REQUESTS_PER_CLIENT,
                    )),
                    "parallel requests",
                );
            },
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

async fn spawn_server() -> Result<BenchServer, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (tx, mut rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = &mut rx => break,
                accept = listener.accept() => {
                    let Ok((mut stream, _peer)) = accept else { continue };
                    tokio::spawn(async move {
                        let mut buf = [0u8; 1024];
                        loop {
                            let n = match stream.read(&mut buf).await {
                                Ok(0) => break,
                                Ok(n) => n,
                                Err(_) => break,
                            };
                            let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
                            let body: &[u8] = if req.starts_with("GET /health") {
                                HEALTH_RESPONSE
                            } else if req.starts_with("GET /providers") {
                                PROVIDERS_RESPONSE
                            } else {
                                NOT_FOUND
                            };
                            if stream.write_all(body).await.is_err() {
                                break;
                            }
                        }
                    });
                }
            }
        }
    });
    Ok(BenchServer {
        url: format!("http://{addr}"),
        shutdown: Some(tx),
        join: Some(handle),
    })
}

async fn serial_requests(
    client: &reqwest::Client,
    url: &str,
    count: usize,
) -> Result<(), reqwest::Error> {
    for _ in 0..count {
        let resp = client.get(url).send().await?;
        let _ = resp.bytes().await?;
    }
    Ok(())
}

async fn parallel_requests(
    client: reqwest::Client,
    url: String,
    clients: usize,
    per_client: usize,
) -> Result<(), reqwest::Error> {
    let mut tasks: JoinSet<Result<(), reqwest::Error>> = JoinSet::new();
    for _ in 0..clients {
        let c = client.clone();
        let u = url.clone();
        tasks.spawn(async move {
            for _ in 0..per_client {
                let resp = c.get(&u).send().await?;
                let _ = resp.bytes().await?;
            }
            Ok(())
        });
    }
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {}
        }
    }
    Ok(())
}

fn must<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

criterion_group!(benches, gateway_request_rps);
criterion_main!(benches);
