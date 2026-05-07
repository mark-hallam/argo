use super::required_env;
use argo_channels::Notification;
use argo_identity::NotifyChannelRef;

pub(super) async fn send_sms(ref_: &NotifyChannelRef, n: &Notification) -> Result<(), String> {
    let sid = required_env("sms", "TWILIO_ACCOUNT_SID")?;
    let token = required_env("sms", "TWILIO_AUTH_TOKEN")?;
    let from = required_env("sms", "TWILIO_FROM")?;
    let base = argo_config::cred::get("TWILIO_API_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://api.twilio.com".into());
    let url = format!(
        "{}/2010-04-01/Accounts/{}/Messages.json",
        base.trim_end_matches('/'),
        sid
    );
    let body = format!(
        "{}\nApprove: {}\nReject: {}\nExpires: {}",
        n.prompt_body, n.approve_url, n.reject_url, n.expiry
    );
    let params = [
        ("From", from.as_str()),
        ("To", ref_.target.as_str()),
        ("Body", body.as_str()),
    ];
    let resp = reqwest::Client::new()
        .post(url)
        .basic_auth(&sid, Some(&token))
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("sms send failed: {e}"))?;
    let status = resp.status();
    let raw = match resp.text().await {
        Ok(body) => body,
        Err(e) => format!("failed to read response body: {e}"),
    };
    if status.is_success() {
        Ok(())
    } else {
        Err(format!("sms returned {status}: {raw}"))
    }
}
