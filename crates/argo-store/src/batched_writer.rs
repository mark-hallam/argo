//! Generic write coalescer for the message + audit hot paths
//! (Stream `claude-perf-store-batched-writes`).
//!
//! The drain loop accepts items via
//! [`tokio::sync::mpsc::UnboundedSender`] and flushes either when
//! the buffer reaches `MAX_BATCH_SIZE` (default 64) or when
//! `MAX_BATCH_LATENCY_MS` elapses since the first buffered item
//! (default 10 ms). Submit via [`BatchedWriter::submit`] resolves
//! only after the flush that included the submitter's item
//! completes — same durability guarantee as the per-row path, but
//! throughput rises because concurrent submitters share one
//! `BEGIN`/`COMMIT`.
//!
//! Env knobs: `ARGO_STORE_BATCH_SIZE`,
//! `ARGO_STORE_BATCH_LATENCY_MS`, and `ARGO_STORE_BATCHED_WRITES=0`
//! (call-site opt-out checked via [`is_batching_enabled`]).
use argo_types::{ArgoError, ArgoResult};
use std::future::{self, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

/// Default flush threshold by buffer size (rows).
pub const MAX_BATCH_SIZE: usize = 64;
/// Default flush threshold by latency (milliseconds).
pub const MAX_BATCH_LATENCY_MS: u64 = 10;

/// `ARGO_STORE_BATCH_SIZE` override; zero / non-numeric falls back
/// to [`MAX_BATCH_SIZE`] (a zero size would wedge the drain loop).
#[must_use]
pub fn batch_size_from_env() -> usize {
    std::env::var("ARGO_STORE_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(MAX_BATCH_SIZE)
}

/// `ARGO_STORE_BATCH_LATENCY_MS` override; non-numeric falls back
/// to [`MAX_BATCH_LATENCY_MS`].
#[must_use]
pub fn batch_latency_from_env() -> Duration {
    let ms = std::env::var("ARGO_STORE_BATCH_LATENCY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(MAX_BATCH_LATENCY_MS);
    Duration::from_millis(ms)
}

/// `false` only when `ARGO_STORE_BATCHED_WRITES=0`; the call-site
/// uses this to bypass the writer and fall back to per-row inserts.
#[must_use]
pub fn is_batching_enabled() -> bool {
    !matches!(
        std::env::var("ARGO_STORE_BATCHED_WRITES").as_deref(),
        Ok("0")
    )
}

/// Boxed async closure stored by [`BatchedWriter`]; the boxing
/// lets one writer type host any caller-provided `Fn`/`Future`.
pub type FlushFn<T> = Arc<
    dyn Fn(Vec<T>) -> Pin<Box<dyn Future<Output = ArgoResult<()>> + Send>> + Send + Sync + 'static,
>;

enum Cmd<T> {
    Item(T, Option<oneshot::Sender<ArgoResult<()>>>),
    Flush(oneshot::Sender<ArgoResult<()>>),
    Shutdown(oneshot::Sender<()>),
}

/// Generic batched writer. `T` is the per-row payload —
/// `MessageItem` for messages, `AuditItem` for audit rows.
///
/// ```no_run
/// # use argo_store::batched_writer::BatchedWriter;
/// # use argo_types::ArgoResult;
/// # async fn doc() -> ArgoResult<()> {
/// let w = BatchedWriter::new(
///     |b: Vec<i32>| async move { tracing::info!(rows = b.len()); Ok(()) },
///     8, std::time::Duration::from_millis(5),
/// );
/// w.submit(42).await?;
/// w.shutdown().await;
/// # Ok(()) }
/// ```
pub struct BatchedWriter<T: Send + 'static> {
    tx: mpsc::UnboundedSender<Cmd<T>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl<T: Send + 'static> BatchedWriter<T> {
    /// Spawn the background drain task and return a handle.
    /// `flush_fn` is called with each drained batch.
    pub fn new<F, Fut>(flush_fn: F, max_size: usize, max_latency: Duration) -> Self
    where
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ArgoResult<()>> + Send + 'static,
    {
        let boxed: FlushFn<T> = Arc::new(move |batch| Box::pin(flush_fn(batch)));
        Self::new_boxed(boxed, max_size, max_latency)
    }

    /// Variant taking a pre-boxed flush fn; useful when the same
    /// `Arc<dyn Fn>` needs to be shared across multiple writers.
    pub fn new_boxed(flush_fn: FlushFn<T>, max_size: usize, max_latency: Duration) -> Self {
        let max_size = max_size.max(1);
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = tokio::spawn(drain_loop(rx, flush_fn, max_size, max_latency));
        Self {
            tx,
            handle: Mutex::new(Some(handle)),
        }
    }

    /// Submit one item and await its commit. The future resolves
    /// with the result of the flush that included this item.
    pub async fn submit(&self, item: T) -> ArgoResult<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Item(item, Some(ack_tx)))
            .map_err(|_| ArgoError::Provider("batched_writer: channel closed".into()))?;
        ack_rx
            .await
            .map_err(|_| ArgoError::Provider("batched_writer: ack dropped".into()))?
    }

    /// Submit one item without awaiting its commit. The caller will
    /// not observe a `flush_fn` error; the row is best-effort.
    pub fn submit_fire_and_forget(&self, item: T) -> ArgoResult<()> {
        self.tx
            .send(Cmd::Item(item, None))
            .map_err(|_| ArgoError::Provider("batched_writer: channel closed".into()))
    }

    /// Force an immediate flush of any buffered rows. Returns once
    /// the in-flight batch (if any) has been written. Tests use this
    /// to drop the latency-window wait before reading rows back.
    pub async fn flush_now(&self) -> ArgoResult<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Flush(ack_tx))
            .map_err(|_| ArgoError::Provider("batched_writer: channel closed".into()))?;
        ack_rx
            .await
            .map_err(|_| ArgoError::Provider("batched_writer: ack dropped".into()))?
    }

    /// Flush pending rows and stop the background task. Idempotent —
    /// re-calling after the task has exited is a no-op.
    pub async fn shutdown(&self) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self.tx.send(Cmd::Shutdown(ack_tx)).is_err() {
            return;
        }
        let _ = ack_rx.await;
        let mut guard = self.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle.await;
        }
    }
}

impl<T: Send + 'static> Drop for BatchedWriter<T> {
    fn drop(&mut self) {
        // Best-effort shutdown signal; we can't `await` from `Drop`
        // so also `abort()` the task to keep the test/bench runtime
        // from accumulating drain tasks across many `Store::open` cycles.
        let (ack_tx, _ack_rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::Shutdown(ack_tx));
        if let Ok(mut guard) = self.handle.try_lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

async fn drain_loop<T: Send + 'static>(
    mut rx: mpsc::UnboundedReceiver<Cmd<T>>,
    flush_fn: FlushFn<T>,
    max_size: usize,
    max_latency: Duration,
) {
    let mut buf: Vec<T> = Vec::with_capacity(max_size);
    let mut acks: Vec<oneshot::Sender<ArgoResult<()>>> = Vec::with_capacity(max_size);
    let mut pending_flush: Option<oneshot::Sender<ArgoResult<()>>> = None;
    let mut pending_shutdown: Option<oneshot::Sender<()>> = None;
    let mut deadline: Option<tokio::time::Instant> = None;

    loop {
        // `pending()` registers no timer slot — an idle drain loop puts
        // zero pressure on the runtime's timer wheel even when many
        // `BatchedWriter`s live concurrently.
        let timer: Pin<Box<dyn Future<Output = ()> + Send>> = match deadline {
            Some(d) => Box::pin(tokio::time::sleep_until(d)),
            None => Box::pin(future::pending::<()>()),
        };
        tokio::pin!(timer);

        tokio::select! {
            biased;
            cmd = rx.recv() => {
                let mut shutdown_now = false;
                let mut flush_now = false;
                if !push_cmd(
                    cmd,
                    &mut buf,
                    &mut acks,
                    &mut pending_flush,
                    &mut pending_shutdown,
                    &mut shutdown_now,
                    &mut flush_now,
                ) {
                    // Channel closed: drain whatever's buffered and exit.
                    if !buf.is_empty() {
                        flush_buffer(&flush_fn, &mut buf, &mut acks).await;
                    }
                    return;
                }
                // Greedy slurp: a burst of concurrent submitters fills the
                // buffer in one tx. Stops at max_size or when the channel
                // momentarily drains.
                let mut channel_drained = false;
                while buf.len() < max_size {
                    match rx.try_recv() {
                        Ok(c) => {
                            if !push_cmd(
                                Some(c),
                                &mut buf,
                                &mut acks,
                                &mut pending_flush,
                                &mut pending_shutdown,
                                &mut shutdown_now,
                                &mut flush_now,
                            ) {
                                break;
                            }
                        }
                        Err(_) => {
                            channel_drained = true;
                            break;
                        }
                    }
                }

                // Flush as soon as the channel temporarily drains — batching
                // only helps when *concurrent* writers pile up. Sequential
                // workloads pay no latency-window cost.
                if buf.len() >= max_size || flush_now || shutdown_now || channel_drained {
                    if !buf.is_empty() {
                        flush_buffer(&flush_fn, &mut buf, &mut acks).await;
                    }
                    deadline = None;
                    if let Some(ack) = pending_flush.take() {
                        let _ = ack.send(Ok(()));
                    }
                } else if !buf.is_empty() && deadline.is_none() {
                    // try_recv kept yielding until we hit max_size; bound the
                    // wait if no further items arrive.
                    deadline = Some(tokio::time::Instant::now() + max_latency);
                }
                if shutdown_now {
                    if let Some(ack) = pending_shutdown.take() {
                        let _ = ack.send(());
                    }
                    return;
                }
            }
            _ = &mut timer, if deadline.is_some() => {
                if !buf.is_empty() {
                    flush_buffer(&flush_fn, &mut buf, &mut acks).await;
                }
                deadline = None;
                if let Some(ack) = pending_flush.take() {
                    let _ = ack.send(Ok(()));
                }
            }
        }
    }
}

/// Apply one received command to the drain-loop buffers. Returns
/// `false` when the channel closed (drain loop must exit). Out-params
/// `flush_now`/`shutdown_now` defer the action so the caller can
/// finish slurping the rest of the channel first.
fn push_cmd<T>(
    cmd: Option<Cmd<T>>,
    buf: &mut Vec<T>,
    acks: &mut Vec<oneshot::Sender<ArgoResult<()>>>,
    pending_flush: &mut Option<oneshot::Sender<ArgoResult<()>>>,
    pending_shutdown: &mut Option<oneshot::Sender<()>>,
    shutdown_now: &mut bool,
    flush_now: &mut bool,
) -> bool {
    match cmd {
        Some(Cmd::Item(item, ack)) => {
            buf.push(item);
            if let Some(a) = ack {
                acks.push(a);
            }
            true
        }
        Some(Cmd::Flush(ack)) => {
            // Coalesce concurrent Flush requests; the most recent
            // ack wins, the prior one's rx sees Err (only tests use
            // this path, behaviour acceptable).
            *pending_flush = Some(ack);
            *flush_now = true;
            true
        }
        Some(Cmd::Shutdown(ack)) => {
            *pending_shutdown = Some(ack);
            *shutdown_now = true;
            true
        }
        None => false,
    }
}

async fn flush_buffer<T: Send + 'static>(
    flush_fn: &FlushFn<T>,
    buf: &mut Vec<T>,
    acks: &mut Vec<oneshot::Sender<ArgoResult<()>>>,
) {
    let batch = std::mem::take(buf);
    let outcome = (flush_fn)(batch).await;
    for a in acks.drain(..) {
        let _ = a.send(clone_result(&outcome));
    }
}

/// `ArgoError` doesn't derive `Clone`; round-trip the `Display`
/// form so each per-row ack carries the same human-readable error
/// string the un-batched path would surface.
fn clone_result(r: &ArgoResult<()>) -> ArgoResult<()> {
    match r {
        Ok(()) => Ok(()),
        Err(e) => Err(ArgoError::Provider(format!("batched flush: {e}"))),
    }
}

#[cfg(test)]
#[path = "batched_writer_tests.rs"]
mod tests;
