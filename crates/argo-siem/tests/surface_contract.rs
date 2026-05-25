use std::sync::{Arc, Mutex};
use std::time::Duration;

use argo_siem::{
    datadog, dispatcher, elastic, sink, spawn_siem_dispatcher, splunk, DatadogLogsSink,
    ElasticSink, SiemDispatcher, SiemDispatcherConfig, SiemEvent, SiemSink, SinkError,
    SplunkHecSink,
};
use argo_store::SessionStore;
use async_trait::async_trait;

#[test]
fn public_surface_exports_sink_modules_and_dispatcher_entrypoints() {
    fn assert_sink<T: SiemSink + Send + Sync>() {}
    type SplunkCtor =
        fn(String, String, Option<String>) -> Result<splunk::SplunkHecSink, SinkError>;
    type ElasticCtor =
        fn(String, String, Option<String>) -> Result<elastic::ElasticSink, SinkError>;
    type DatadogCtor = fn(
        String,
        String,
        Option<String>,
        Option<String>,
    ) -> Result<datadog::DatadogLogsSink, SinkError>;
    type DispatcherSpawn = fn(
        Arc<SessionStore>,
        Arc<dyn SiemSink>,
        SiemDispatcherConfig,
    ) -> tokio::task::JoinHandle<()>;

    assert_sink::<SplunkHecSink>();
    assert_sink::<ElasticSink>();
    assert_sink::<DatadogLogsSink>();

    let _splunk_new: SplunkCtor = splunk::SplunkHecSink::new;
    let _elastic_new: ElasticCtor = elastic::ElasticSink::new;
    let _datadog_new: DatadogCtor = datadog::DatadogLogsSink::new;
    let _spawn: DispatcherSpawn = spawn_siem_dispatcher;

    assert_eq!(
        dispatcher::SiemDispatcherConfig::default().interval,
        Duration::from_secs(dispatcher::DEFAULT_INTERVAL_SECS)
    );
    assert_eq!(dispatcher::DEFAULT_INTERVAL_SECS, 15);
    assert_eq!(
        std::mem::size_of::<sink::SiemEvent>(),
        std::mem::size_of::<SiemEvent>()
    );
}

#[test]
fn siem_event_json_shape_stays_canonical() {
    let event = sample_event(42);
    let json = serde_json::to_value(&event).expect("serialize siem event");
    let object = json.as_object().expect("siem event object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();

    assert_eq!(
        keys,
        vec!["created_at", "id", "kind", "payload", "seq", "session_id"]
    );
    assert_eq!(json["seq"], 42);
    assert_eq!(json["id"], "audit-42");
    assert_eq!(json["session_id"], "session-42");
    assert_eq!(json["kind"], "approval.replied");
    assert_eq!(json["payload"]["tenant_id"], "tenant-a");
    assert_eq!(json["payload"]["decision"], "approve");
    assert_eq!(json["created_at"], 1_700_000_042);

    let decoded: SiemEvent = serde_json::from_value(json).expect("deserialize siem event");
    assert_eq!(decoded.seq, event.seq);
    assert_eq!(decoded.id, event.id);
    assert_eq!(decoded.session_id, event.session_id);
    assert_eq!(decoded.kind, event.kind);
    assert_eq!(decoded.payload, event.payload);
    assert_eq!(decoded.created_at, event.created_at);
}

#[test]
fn sink_formatters_emit_stable_request_bodies_without_network() {
    let events = [sample_event(1), sample_event(2)];

    let splunk = SplunkHecSink::new(
        "https://splunk.example.com/".to_string(),
        "splunk-token".to_string(),
        None,
    )
    .expect("splunk sink");
    assert_eq!(splunk.backend(), "splunk");
    assert_eq!(splunk.endpoint(), "https://splunk.example.com");
    assert_eq!(splunk.max_batch(), 500);
    let splunk_lines: Vec<serde_json::Value> = splunk
        .encode_batch(&events)
        .trim_end()
        .lines()
        .map(|line| serde_json::from_str(line).expect("splunk hec line"))
        .collect();
    assert_eq!(splunk_lines.len(), 2);
    assert_eq!(splunk_lines[0]["time"], 1_700_000_001);
    assert_eq!(splunk_lines[0]["source"], "argo");
    assert_eq!(splunk_lines[0]["sourcetype"], "argo:audit");
    assert_eq!(splunk_lines[0]["event"]["seq"], 1);
    assert_eq!(splunk_lines[0]["event"]["payload"]["tenant_id"], "tenant-a");
    assert!(!format!("{splunk:?}").contains("splunk-token"));

    let elastic = ElasticSink::new(
        "https://elastic.example.com/".to_string(),
        "ApiKey opaque".to_string(),
        Some("audit-index".to_string()),
    )
    .expect("elastic sink");
    assert_eq!(elastic.backend(), "elastic");
    assert_eq!(elastic.endpoint(), "https://elastic.example.com");
    assert_eq!(elastic.max_batch(), 1000);
    let elastic_lines: Vec<serde_json::Value> = elastic
        .encode_batch(&events)
        .trim_end()
        .lines()
        .map(|line| serde_json::from_str(line).expect("elastic bulk line"))
        .collect();
    assert_eq!(elastic_lines.len(), 4);
    assert_eq!(elastic_lines[0]["index"]["_index"], "audit-index");
    assert_eq!(elastic_lines[1]["seq"], 1);
    assert_eq!(elastic_lines[1]["@timestamp"], 1_700_000_001);
    assert_eq!(elastic_lines[3]["payload"]["tenant_id"], "tenant-a");
    assert!(!format!("{elastic:?}").contains("opaque"));

    let datadog = DatadogLogsSink::new(
        "https://http-intake.logs.datadoghq.com/api/v2/logs".to_string(),
        "dd-api-key".to_string(),
        None,
        None,
    )
    .expect("datadog sink");
    assert_eq!(datadog.backend(), "datadog");
    assert_eq!(
        datadog.endpoint(),
        "https://http-intake.logs.datadoghq.com/api/v2/logs"
    );
    assert_eq!(datadog.max_batch(), 1000);
    let datadog_body: serde_json::Value =
        serde_json::from_str(&datadog.encode_batch(&events)).expect("datadog json array");
    let datadog_rows = datadog_body.as_array().expect("datadog array");
    assert_eq!(datadog_rows.len(), 2);
    assert_eq!(datadog_rows[0]["ddsource"], "argo");
    assert_eq!(datadog_rows[0]["service"], "argo-gateway");
    assert_eq!(
        datadog_rows[0]["message"],
        "argo audit kind=approval.replied seq=1"
    );
    assert_eq!(datadog_rows[0]["timestamp_ms"], 1_700_000_001_000_i64);
    assert_eq!(datadog_rows[1]["payload"]["decision"], "approve");
    assert!(!format!("{datadog:?}").contains("dd-api-key"));
}

#[tokio::test]
async fn dispatcher_persists_high_water_cursor_by_backend_and_endpoint() {
    let store = open_store().await;
    insert_audit(&store, "approval.replied", 1).await;
    insert_audit(&store, "approval.replied", 2).await;
    insert_audit(&store, "worker.started", 3).await;

    let sink = RecordingSink::new("splunk", "https://siem.example.com", 2);
    let dispatcher = SiemDispatcher::new(
        Arc::clone(&store),
        sink.clone(),
        SiemDispatcherConfig::default(),
    );
    dispatcher.ensure_schema().await.expect("cursor schema");

    assert_eq!(dispatcher.load_cursor().await.expect("initial cursor"), 0);
    assert_eq!(dispatcher.sweep_once().await.expect("first sweep"), Some(2));
    assert_eq!(
        dispatcher.load_cursor().await.expect("cursor after first"),
        2
    );
    assert_eq!(sink.shipped_seqs(), vec![vec![1, 2]]);

    assert_eq!(
        dispatcher.sweep_once().await.expect("second sweep"),
        Some(3)
    );
    assert_eq!(
        dispatcher.load_cursor().await.expect("cursor after second"),
        3
    );
    assert_eq!(sink.shipped_seqs(), vec![vec![1, 2], vec![3]]);

    let restarted_sink = RecordingSink::new("splunk", "https://siem.example.com", 10);
    let restarted = SiemDispatcher::new(
        Arc::clone(&store),
        restarted_sink.clone(),
        SiemDispatcherConfig::default(),
    );
    restarted
        .ensure_schema()
        .await
        .expect("cursor schema restart");
    assert_eq!(restarted.load_cursor().await.expect("restart cursor"), 3);
    assert_eq!(restarted.sweep_once().await.expect("restart empty"), None);
    assert_eq!(restarted_sink.shipped_seqs(), Vec::<Vec<i64>>::new());

    insert_audit(&store, "approval.replied", 4).await;
    assert_eq!(
        restarted.sweep_once().await.expect("post-restart sweep"),
        Some(4)
    );
    assert_eq!(restarted_sink.shipped_seqs(), vec![vec![4]]);
}

#[test]
fn sink_error_strings_preserve_retry_classification_words() {
    assert_eq!(
        SinkError::Transient("timeout".into()).to_string(),
        "transient: timeout"
    );
    assert_eq!(
        SinkError::Permanent("bad request".into()).to_string(),
        "permanent: bad request"
    );
    assert_eq!(
        SinkError::Config("missing token".into()).to_string(),
        "config: missing token"
    );
}

fn sample_event(seq: i64) -> SiemEvent {
    SiemEvent {
        seq,
        id: format!("audit-{seq}"),
        session_id: Some(format!("session-{seq}")),
        kind: "approval.replied".to_string(),
        payload: serde_json::json!({
            "tenant_id": "tenant-a",
            "approval_id": format!("approval-{seq}"),
            "decision": "approve",
        }),
        created_at: 1_700_000_000 + seq,
    }
}

struct RecordingSink {
    backend: &'static str,
    endpoint: String,
    max_batch: usize,
    shipped: Mutex<Vec<Vec<SiemEvent>>>,
}

impl RecordingSink {
    fn new(backend: &'static str, endpoint: &str, max_batch: usize) -> Arc<Self> {
        Arc::new(Self {
            backend,
            endpoint: endpoint.to_string(),
            max_batch,
            shipped: Mutex::new(Vec::new()),
        })
    }

    fn shipped_seqs(&self) -> Vec<Vec<i64>> {
        self.shipped
            .lock()
            .expect("shipped lock")
            .iter()
            .map(|batch| batch.iter().map(|event| event.seq).collect())
            .collect()
    }
}

#[async_trait]
impl SiemSink for RecordingSink {
    fn backend(&self) -> &'static str {
        self.backend
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn max_batch(&self) -> usize {
        self.max_batch
    }

    async fn ship(&self, events: Vec<SiemEvent>) -> Result<(), SinkError> {
        self.shipped.lock().expect("shipped lock").push(events);
        Ok(())
    }
}

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    Arc::new(store)
}

async fn insert_audit(store: &SessionStore, kind: &str, created_at: i64) {
    let mut id = [0_u8; 16];
    id[..8].copy_from_slice(&created_at.to_be_bytes());
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at) \
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_slice())
    .bind(kind)
    .bind(serde_json::json!({"created_at": created_at}).to_string())
    .bind(vec![0_u8; 32])
    .bind(vec![1_u8; 32])
    .bind(created_at)
    .execute(store.pool())
    .await
    .expect("insert audit row");
}
