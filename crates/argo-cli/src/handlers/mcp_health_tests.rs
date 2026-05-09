use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Default)]
struct HealthState {
    last_seen: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

fn server(tenant_id: &str, server_id: &str) -> RegisteredMcpServer {
    RegisteredMcpServer {
        key: ServerKey(tenant_id.into(), server_id.into()),
        declaration: McpServerDeclaration {
            transport: "stdio".into(),
            command: "fixture".into(),
            args: Vec::new(),
        },
    }
}

#[derive(Default)]
struct FakeRegistry {
    servers: Vec<RegisteredMcpServer>,
    states: Mutex<HashMap<ServerKey, HealthState>>,
}

impl FakeRegistry {
    fn new(servers: Vec<RegisteredMcpServer>) -> Self {
        Self {
            servers,
            states: Mutex::new(HashMap::new()),
        }
    }

    async fn state(&self, key: &ServerKey) -> HealthState {
        self.states
            .lock()
            .await
            .get(key)
            .cloned()
            .unwrap_or_default()
    }
}

#[async_trait]
impl McpRegistry for FakeRegistry {
    async fn list_servers(&self) -> ArgoResult<Vec<RegisteredMcpServer>> {
        Ok(self.servers.clone())
    }

    async fn record_success(&self, key: &ServerKey, at: DateTime<Utc>) -> ArgoResult<()> {
        self.states.lock().await.insert(
            key.clone(),
            HealthState {
                last_seen: Some(at),
                last_error: None,
            },
        );
        Ok(())
    }

    async fn record_failure(&self, key: &ServerKey, error: &str) -> ArgoResult<()> {
        self.states.lock().await.insert(
            key.clone(),
            HealthState {
                last_seen: None,
                last_error: Some(error.into()),
            },
        );
        Ok(())
    }
}

#[derive(Default)]
struct FakeProbe {
    outcomes: Mutex<HashMap<ServerKey, VecDeque<Result<(), String>>>>,
    calls: Mutex<HashMap<ServerKey, usize>>,
    inflight: AtomicUsize,
    max_seen: AtomicUsize,
    delay: Duration,
}

impl FakeProbe {
    fn with_delay(delay: Duration) -> Self {
        Self {
            delay,
            ..Self::default()
        }
    }

    async fn push_failure(&self, key: &ServerKey, error: &str) {
        self.outcomes
            .lock()
            .await
            .entry(key.clone())
            .or_default()
            .push_back(Err(error.into()));
    }

    async fn call_count(&self, key: &ServerKey) -> usize {
        self.calls.lock().await.get(key).copied().unwrap_or(0)
    }

    async fn total_calls(&self) -> usize {
        self.calls.lock().await.values().sum()
    }

    fn max_concurrent_seen(&self) -> usize {
        self.max_seen.load(Ordering::SeqCst)
    }

    fn note_inflight(&self) {
        let current = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
        let mut seen = self.max_seen.load(Ordering::SeqCst);
        while current > seen {
            match self
                .max_seen
                .compare_exchange(seen, current, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => break,
                Err(next) => seen = next,
            }
        }
    }
}

#[async_trait]
impl McpHealthProbe for FakeProbe {
    async fn ping(&self, server: &RegisteredMcpServer, _timeout: Duration) -> ArgoResult<()> {
        self.note_inflight();
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        *self
            .calls
            .lock()
            .await
            .entry(server.key.clone())
            .or_default() += 1;
        let outcome = self
            .outcomes
            .lock()
            .await
            .get_mut(&server.key)
            .and_then(VecDeque::pop_front)
            .unwrap_or(Ok(()));
        self.inflight.fetch_sub(1, Ordering::SeqCst);
        outcome.map_err(ArgoError::Provider)
    }
}

fn task(
    registry: Arc<FakeRegistry>,
    probe: Arc<FakeProbe>,
    max_concurrent: usize,
) -> HealthCheckTask {
    HealthCheckTask::new(
        Duration::from_millis(20),
        Duration::from_millis(100),
        max_concurrent,
        registry,
        probe,
    )
}

#[tokio::test]
async fn tick_fans_out_one_ping_per_registered_server() {
    let servers = vec![server("t1", "a"), server("t1", "b"), server("t1", "c")];
    let registry = Arc::new(FakeRegistry::new(servers));
    let probe = Arc::new(FakeProbe::default());
    let summary = task(registry, probe.clone(), 8).run_once().await.unwrap();
    assert_eq!(summary.attempted, 3);
    assert_eq!(summary.succeeded, 3);
    assert_eq!(probe.total_calls().await, 3);
}

#[tokio::test]
async fn successful_ping_updates_last_seen() {
    let srv = server("t1", "ok");
    let registry = Arc::new(FakeRegistry::new(vec![srv.clone()]));
    let probe = Arc::new(FakeProbe::default());
    task(registry.clone(), probe, 8).run_once().await.unwrap();
    let state = registry.state(&srv.key).await;
    assert!(state.last_seen.is_some());
    assert!(state.last_error.is_none());
}

#[tokio::test]
async fn failed_ping_updates_last_error() {
    let srv = server("t1", "dead");
    let registry = Arc::new(FakeRegistry::new(vec![srv.clone()]));
    let probe = Arc::new(FakeProbe::default());
    probe.push_failure(&srv.key, "connection refused").await;
    let summary = task(registry.clone(), probe, 8).run_once().await.unwrap();
    assert_eq!(summary.failed, 1);
    let state = registry.state(&srv.key).await;
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|err| err.contains("connection refused")));
    assert!(state.last_seen.is_none());
}

#[test]
fn exponential_backoff_caps_at_five_minutes() {
    assert_eq!(backoff_delay(1), Duration::from_secs(1));
    assert_eq!(backoff_delay(2), Duration::from_secs(2));
    assert_eq!(backoff_delay(4), Duration::from_secs(8));
    assert_eq!(backoff_delay(20), Duration::from_secs(300));
}

#[tokio::test]
async fn max_concurrent_is_honored() {
    let servers: Vec<_> = (0..6).map(|i| server("t1", &format!("s{i}"))).collect();
    let registry = Arc::new(FakeRegistry::new(servers));
    let probe = Arc::new(FakeProbe::with_delay(Duration::from_millis(25)));
    let summary = task(registry, probe.clone(), 2).run_once().await.unwrap();
    assert_eq!(summary.attempted, 6);
    assert!(probe.max_concurrent_seen() <= 2);
}

#[tokio::test]
async fn tenant_isolation_keeps_same_server_id_separate() {
    let left = server("tenant-a", "gmail");
    let right = server("tenant-b", "gmail");
    let registry = Arc::new(FakeRegistry::new(vec![left.clone(), right.clone()]));
    let probe = Arc::new(FakeProbe::default());
    probe.push_failure(&left.key, "left failed").await;
    task(registry.clone(), probe.clone(), 8)
        .run_once()
        .await
        .unwrap();
    assert_eq!(probe.call_count(&left.key).await, 1);
    assert_eq!(probe.call_count(&right.key).await, 1);
    assert!(registry
        .state(&left.key)
        .await
        .last_error
        .as_deref()
        .is_some_and(|err| err.contains("left failed")));
    assert!(registry.state(&right.key).await.last_seen.is_some());
}

#[tokio::test]
async fn graceful_shutdown_via_cancellation_token() {
    let registry = Arc::new(FakeRegistry::new(Vec::new()));
    let probe = Arc::new(FakeProbe::default());
    let health = Arc::new(task(registry, probe, 8));
    let token = CancellationToken::new();
    let run_token = token.clone();
    let handle = tokio::spawn(async move { health.run(run_token).await });
    token.cancel();
    let finished = tokio::time::timeout(Duration::from_millis(200), handle)
        .await
        .expect("task exits after cancellation")
        .expect("join succeeds");
    assert!(finished.is_ok());
}
