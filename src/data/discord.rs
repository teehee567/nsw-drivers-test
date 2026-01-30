use chrono::Utc;

pub async fn notify_403_blocked(
    webhook_url: &str,
    proxy: &str,
    status_code: u16,
    response_body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = if response_body.len() > 1000 { &response_body[..1000] } else { response_body };
    let escaped = body.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    let ts = chrono::Utc::now().to_rfc3339();
    
    let payload = format!(
        r#"{{"content":null,"embeds":[{{"title":"Proxy Block Detected","description":"@everyone","color":5814783,"fields":[{{"name":"Proxy Ip","value":"{}"}},{{"name":"Response code","value":"{}"}},{{"name":"Response Dump","value":"{}"}}],"timestamp":"{}"}}],"attachments":[]}}"#,
        proxy, status_code, escaped, ts
    );

    reqwest::Client::new()
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?
        .error_for_status()?;

    log::info!("Discord notification sent for blocked proxy: {}", proxy);
    Ok(())
}
