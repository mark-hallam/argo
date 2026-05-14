use super::*;
use crate::{ChannelError, ChannelId, ChannelResult, DeliveryReceipt, InboundEvent};
use parking_lot::Mutex;

#[derive(Default)]
struct FakeChannel {
    id: String,
    sent: Mutex<Vec<Notification>>,
    fail: bool,
}

impl FakeChannel {
    fn ok(id: &str) -> Arc<Self> {
        Arc::new(Self {
            id: id.into(),
            sent: Mutex::new(Vec::new()),
            fail: false,
        })
    }

    fn fail(id: &str) -> Arc<Self> {
        Arc::new(Self {
            id: id.into(),
            sent: Mutex::new(Vec::new()),
            fail: true,
        })
    }

    fn sent_len(&self) -> usize {
        self.sent.lock().len()
    }
}

#[async_trait]
impl NotifyChannel for FakeChannel {
    fn id(&self) -> ChannelId {
        ChannelId::new(&self.id)
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        self.sent.lock().push(notification);
        if self.fail {
            return Err(ChannelError::Provider("boom".into()));
        }
        Ok(receipt(self.id()))
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }
}

fn alert(kind: AnomalyKind, severity: Severity) -> AnomalyAlert {
    AnomalyAlert {
        kind,
        severity,
        message: "test alert".into(),
        context: BTreeMap::new(),
    }
}

#[tokio::test]
async fn dispatch_fans_out_to_all_channels_without_route() {
    let a = FakeChannel::ok("slack");
    let b = FakeChannel::ok("email");
    let dispatch = MultiChannelDispatch::new(
        registry(vec![a.clone(), b.clone()]),
        BTreeMap::new(),
        Severity::Warn,
    );
    dispatch
        .dispatch(alert(AnomalyKind::ProviderStorm, Severity::Warn))
        .await
        .unwrap();
    assert_eq!(a.sent_len(), 1);
    assert_eq!(b.sent_len(), 1);
}

#[tokio::test]
async fn severity_filter_drops_below_threshold() {
    let a = FakeChannel::ok("slack");
    let dispatch = MultiChannelDispatch::new(
        registry(vec![a.clone()]),
        BTreeMap::new(),
        Severity::Critical,
    );
    dispatch
        .dispatch(alert(AnomalyKind::ProviderStorm, Severity::Warn))
        .await
        .unwrap();
    assert_eq!(a.sent_len(), 0);
}

#[tokio::test]
async fn missing_channel_route_is_skipped() {
    let a = FakeChannel::ok("slack");
    let mut routing = BTreeMap::new();
    routing.insert(
        AnomalyKind::BudgetCeiling,
        vec!["missing".into(), "slack".into()],
    );
    let dispatch = MultiChannelDispatch::new(registry(vec![a.clone()]), routing, Severity::Warn);
    dispatch
        .dispatch(alert(AnomalyKind::BudgetCeiling, Severity::Warn))
        .await
        .unwrap();
    assert_eq!(a.sent_len(), 1);
}

#[tokio::test]
async fn failing_channel_does_not_stop_later_channels() {
    let bad = FakeChannel::fail("bad");
    let good = FakeChannel::ok("good");
    let dispatch = MultiChannelDispatch::new(
        registry(vec![bad.clone(), good.clone()]),
        BTreeMap::new(),
        Severity::Warn,
    );
    let err = dispatch
        .dispatch(alert(AnomalyKind::ChainBreak, Severity::Critical))
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("bad"));
    assert_eq!(bad.sent_len(), 1);
    assert_eq!(good.sent_len(), 1);
}

#[tokio::test]
async fn kind_routing_limits_channel_set() {
    let slack = FakeChannel::ok("slack");
    let email = FakeChannel::ok("email");
    let mut routing = BTreeMap::new();
    routing.insert(AnomalyKind::GatewayBootFail, vec!["email".into()]);
    let dispatch = MultiChannelDispatch::new(
        registry(vec![slack.clone(), email.clone()]),
        routing,
        Severity::Warn,
    );
    dispatch
        .dispatch(alert(AnomalyKind::GatewayBootFail, Severity::Critical))
        .await
        .unwrap();
    assert_eq!(slack.sent_len(), 0);
    assert_eq!(email.sent_len(), 1);
}

#[tokio::test]
async fn disabled_config_returns_noop_dispatch() {
    let ch = FakeChannel::ok("slack");
    let cfg = AnomalyAlertingConfig {
        enabled: false,
        ..Default::default()
    };
    let dispatch = MultiChannelDispatch::from_config(&cfg, registry(vec![ch.clone()]));
    dispatch
        .dispatch(alert(AnomalyKind::ProviderStorm, Severity::Critical))
        .await
        .unwrap();
    assert_eq!(ch.sent_len(), 0);
}
