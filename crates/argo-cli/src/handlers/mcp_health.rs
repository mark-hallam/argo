use argo_mcp::McpClient;
use argo_types::{ArgoError, ArgoResult};
use argo_vertical::McpServerDeclaration;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use super::worker_actions_executor_mcp::McpPool;

const DEFAULT_TICK_SECS: u64 = 60;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const DEFAULT_MAX_CONCURRENT: usize = 8;
const MAX_BACKOFF: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ServerKey(String, String);

impl ServerKey {
    fn pool_key(&self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

#[derive(Debug, Clone)]
struct RegisteredMcpServer {
    key: ServerKey,
    declaration: McpServerDeclaration,
}

#[async_trait]
trait McpRegistry: Send + Sync {
    async fn list_servers(&self) -> ArgoResult<Vec<RegisteredMcpServer>>;
    async fn record_success(&self, key: &ServerKey, at: DateTime<Utc>) -> ArgoResult<()>;
    async fn record_failure(&self, key: &ServerKey, error: &str) -> ArgoResult<()>;
}

#[async_trait]
trait McpHealthProbe: Send + Sync {
    async fn ping(&self, server: &RegisteredMcpServer, timeout: Duration) -> ArgoResult<()>;
}

struct HealthCheckTask {
    tick_interval: Duration,
    ping_timeout: Duration,
    max_concurrent: usize,
    registry: Arc<dyn McpRegistry>,
    probe: Arc<dyn McpHealthProbe>,
    backoff: Mutex<HashMap<ServerKey, (u32, Instant)>>,
}

impl HealthCheckTask {
    fn new(
        tick_interval: Duration,
        ping_timeout: Duration,
        max_concurrent: usize,
        registry: Arc<dyn McpRegistry>,
        probe: Arc<dyn McpHealthProbe>,
    ) -> Self {
        Self {
            tick_interval,
            ping_timeout,
            max_concurrent,
            registry,
            probe,
            backoff: Mutex::new(HashMap::new()),
        }
    }

    async fn run(&self, shutdown: CancellationToken) -> ArgoResult<()> {
        let mut tick = tokio::time::interval(self.tick_interval);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = tick.tick() => {
                    let summary = self.run_once().await?;
                    tracing::debug!(
                        target: "argo",
                        attempted = summary.attempted,
                        succeeded = summary.succeeded,
                        failed = summary.failed,
                        "mcp.health tick"
                    );
                }
            }
        }
    }

    async fn run_once(&self) -> ArgoResult<PingSummary> {
        let servers = self.registry.list_servers().await?;
        let now = Instant::now();
        let backoff = self.backoff.lock().await;
        let due: Vec<_> = servers
            .into_iter()
            .filter(|s| backoff.get(&s.key).map(|b| b.1 <= now).unwrap_or(true))
            .collect();
        drop(backoff);
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent.max(1)));
        let mut joins = JoinSet::new();
        for server in due {
            let permit = Arc::clone(&semaphore);
            let registry = Arc::clone(&self.registry);
            let probe = Arc::clone(&self.probe);
            let timeout = self.ping_timeout;
            joins.spawn(async move {
                let _permit = permit.acquire_owned().await.map_err(|e| {
                    ArgoError::Provider(format!("mcp health semaphore closed: {e}"))
                })?;
                ping_one(registry, probe, server, timeout).await
            });
        }

        let mut summary = PingSummary::default();
        while let Some(joined) = joins.join_next().await {
            let (key, success) =
                joined.map_err(|e| ArgoError::Provider(format!("mcp health task: {e}")))??;
            summary.attempted += 1;
            if success {
                summary.succeeded += 1;
            } else {
                summary.failed += 1;
            }
            self.update_backoff(key, success).await;
        }
        Ok(summary)
    }

    async fn update_backoff(&self, key: ServerKey, success: bool) {
        let mut backoff = self.backoff.lock().await;
        if success {
            backoff.remove(&key);
            return;
        }
        let entry = backoff.entry(key).or_insert((0, Instant::now()));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = Instant::now() + backoff_delay(entry.0);
    }
}

#[derive(Debug, Default)]
struct PingSummary {
    attempted: usize,
    succeeded: usize,
    failed: usize,
}

fn backoff_delay(failures: u32) -> Duration {
    let exp = failures.saturating_sub(1).min(16);
    Duration::from_secs((1_u64 << exp).min(MAX_BACKOFF.as_secs()))
}

async fn ping_one(
    registry: Arc<dyn McpRegistry>,
    probe: Arc<dyn McpHealthProbe>,
    server: RegisteredMcpServer,
    timeout: Duration,
) -> ArgoResult<(ServerKey, bool)> {
    let started = Instant::now();
    let key = server.key.clone();
    match probe.ping(&server, timeout).await {
        Ok(()) => {
            registry.record_success(&key, Utc::now()).await?;
            tracing::info!(
                target: "argo",
                server_id = %key.1,
                status = "ok",
                latency_ms = started.elapsed().as_millis() as u64,
                "mcp.health"
            );
            Ok((key, true))
        }
        Err(e) => {
            let err = e.to_string();
            registry.record_failure(&key, &err).await?;
            tracing::info!(
                target: "argo",
                server_id = %key.1,
                status = "error",
                latency_ms = started.elapsed().as_millis() as u64,
                error = %err,
                "mcp.health"
            );
            Ok((key, false))
        }
    }
}

struct PooledMcpProbe {
    pool: Arc<McpPool>,
}

#[async_trait]
impl McpHealthProbe for PooledMcpProbe {
    async fn ping(&self, server: &RegisteredMcpServer, timeout: Duration) -> ArgoResult<()> {
        if server.declaration.transport != "stdio" {
            return Err(ArgoError::Provider(format!(
                "MCP server '{}' uses unsupported transport '{}'",
                server.key.1, server.declaration.transport
            )));
        }
        let pool_key = server.key.pool_key();
        let client = {
            let mut pool = self.pool.lock().await;
            if let Some(client) = pool.get(&pool_key) {
                client.clone()
            } else {
                let connected = tokio::time::timeout(
                    timeout,
                    McpClient::connect_stdio_with_timeout(
                        &server.declaration.command,
                        &server.declaration.args,
                        timeout,
                    ),
                )
                .await
                .map_err(|_| ArgoError::Provider("MCP health connect timed out".into()))?
                .map_err(|e| ArgoError::Provider(format!("MCP health connect: {e}")))?;
                let client = Arc::new(connected);
                pool.insert(pool_key, client.clone());
                client
            }
        };
        let listed = tokio::time::timeout(timeout, client.list_tools()).await;
        match listed {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => {
                self.pool.lock().await.remove(&server.key.pool_key());
                Err(ArgoError::Provider(format!("MCP health list_tools: {e}")))
            }
            Err(_) => {
                self.pool.lock().await.remove(&server.key.pool_key());
                Err(ArgoError::Provider(
                    "MCP health list_tools timed out".into(),
                ))
            }
        }
    }
}

struct StaticManifestMcpRegistry;

#[async_trait]
impl McpRegistry for StaticManifestMcpRegistry {
    async fn list_servers(&self) -> ArgoResult<Vec<RegisteredMcpServer>> {
        let (registry, _errors) = argo_vertical::load_dir(&argo_vertical::default_dir())?;
        let mut out = Vec::new();
        for (vertical_id, loaded) in registry.list() {
            if let Some(servers) = &loaded.manifest.mcp_servers {
                for (server_id, declaration) in &servers.servers {
                    out.push(RegisteredMcpServer {
                        key: ServerKey(vertical_id.to_string(), server_id.clone()),
                        declaration: declaration.clone(),
                    });
                }
            }
        }
        Ok(out)
    }

    async fn record_success(&self, _key: &ServerKey, _at: DateTime<Utc>) -> ArgoResult<()> {
        Ok(())
    }

    async fn record_failure(&self, _key: &ServerKey, _error: &str) -> ArgoResult<()> {
        Ok(())
    }
}

pub(crate) struct HealthCheckGuard(CancellationToken, JoinHandle<()>);

impl Drop for HealthCheckGuard {
    fn drop(&mut self) {
        self.0.cancel();
        self.1.abort();
    }
}

pub(crate) fn spawn_from_env() -> Option<HealthCheckGuard> {
    if std::env::var("ARGO_MCP_HEALTH_CHECK").ok().as_deref() != Some("1") {
        return None;
    }
    let tick_interval = Duration::from_millis(env_parse(
        "ARGO_MCP_HEALTH_INTERVAL_MS",
        DEFAULT_TICK_SECS * 1000,
    ));
    let ping_timeout = Duration::from_millis(env_parse(
        "ARGO_MCP_HEALTH_TIMEOUT_MS",
        DEFAULT_TIMEOUT_SECS * 1000,
    ));
    let max_concurrent = env_parse("ARGO_MCP_HEALTH_MAX_CONCURRENT", DEFAULT_MAX_CONCURRENT);
    let registry = Arc::new(StaticManifestMcpRegistry);
    let probe = Arc::new(PooledMcpProbe {
        pool: Arc::new(McpPool::default()),
    });
    let task = Arc::new(HealthCheckTask::new(
        tick_interval,
        ping_timeout,
        max_concurrent,
        registry,
        probe,
    ));
    let token = CancellationToken::new();
    let run_token = token.clone();
    eprintln!(
        "argo gateway: mcp health check enabled (interval={}ms, timeout={}ms, max_concurrent={})",
        tick_interval.as_millis(),
        ping_timeout.as_millis(),
        max_concurrent
    );
    let handle = tokio::spawn(async move {
        if let Err(e) = task.run(run_token).await {
            tracing::warn!(target: "argo", error = %e, "mcp.health task stopped");
        }
    });
    Some(HealthCheckGuard(token, handle))
}

fn env_parse<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + PartialOrd + Copy + From<u8>,
{
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<T>().ok())
        .filter(|v| *v > T::from(0))
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "mcp_health_tests.rs"]
mod tests;
