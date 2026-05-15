use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn serve_once(assert_req: fn(&str), body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0; 8192];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).to_string();
        assert_req(&req);
        let status = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
        let response = format!("{status}Content-Length: {}\r\n\r\n{body}", body.len());
        stream.write_all(response.as_bytes()).unwrap();
    });
    format!("http://{addr}")
}

fn auth_seen(req: &str) {
    assert!(req
        .to_ascii_lowercase()
        .contains("authorization: bearer sk_test"));
}

#[tokio::test]
async fn advertises_full_stripe_tool_set() {
    let names: Vec<_> = StripeService
        .list_tools()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert_eq!(names.len(), 14);
    assert!(names.contains(&"stripe.create_charge".into()));
    assert!(names.contains(&"stripe.list_payment_intents".into()));
    assert!(names.contains(&"stripe.create_refund".into()));
    assert!(names.contains(&"stripe.refund_charge".into()));
    assert!(names.contains(&"stripe.list_subscriptions".into()));
}

#[tokio::test]
async fn list_customers_uses_bearer_auth_and_query() {
    let base = serve_once(
        |req| {
            auth_seen(req);
            assert!(req.starts_with("GET /v1/customers?limit=2 "));
        },
        r#"{"object":"list","data":[]}"#,
    );
    let client = StripeRestClient::new("sk_test".into(), base).unwrap();
    let out = client
        .call("stripe.list_customers", &serde_json::json!({"limit": 2}))
        .await
        .unwrap();
    assert_eq!(out["object"], "list");
}

#[tokio::test]
async fn create_payment_intent_posts_form() {
    let base = serve_once(
        |req| {
            auth_seen(req);
            assert!(req.starts_with("POST /v1/payment_intents "));
            assert!(req.contains("amount=5000&currency=usd&customer=cus_123"));
        },
        r#"{"id":"pi_123","object":"payment_intent"}"#,
    );
    let client = StripeRestClient::new("sk_test".into(), base).unwrap();
    let out = client
        .call(
            "stripe.create_payment_intent",
            &serde_json::json!({
                "amount": 5000,
                "currency": "usd",
                "customer": "cus_123"
            }),
        )
        .await
        .unwrap();
    assert_eq!(out["id"], "pi_123");
}

#[tokio::test]
async fn rejects_unsafe_path_ids() {
    let client = StripeRestClient::new("sk_test".into(), "http://127.0.0.1:9".into()).unwrap();
    let err = client
        .call(
            "stripe.get_customer",
            &serde_json::json!({"id": "../cus_123"}),
        )
        .await
        .unwrap_err();
    assert!(err.contains("unsafe Stripe id"));
}

#[tokio::test]
async fn mutating_tools_require_approval() {
    for tool in StripeService.list_tools() {
        let approval = tool.input_schema["approval_required"].as_bool().unwrap();
        assert_eq!(
            approval,
            tool.name.starts_with("stripe.create_")
                || tool.name == "stripe.refund_charge"
                || tool.name == "stripe.cancel_subscription"
        );
    }
}
