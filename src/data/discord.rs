use chrono::Utc;
use std::error::Error;
use serde_json::json;

pub async fn notify_403_blocked(
    webhook_url: &str,
    proxy: &str,
    status_code: u16,
    response_body: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let body = if response_body.len() > 1000 { &response_body[..1000] } else { response_body };
    let ts = Utc::now().to_rfc3339();

    let payload = json!({
        "content": null,
        "embeds": [{
            "title": "Proxy Block Detected",
            "description": "<@here>",
            "color": 5814783,
            "fields": [
                {"name": "Proxy Ip", "value": proxy},
                {"name": "Response code", "value": status_code.to_string()},
                {"name": "Response Dump", "value": body}
            ],
            "timestamp": ts
        }],
        "attachments": []
    }).to_string();

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

pub async fn notify_scrape_blocked(
    webhook_url: &str,
    failed_locations: usize,
    max_retries: u64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let ts = Utc::now().to_rfc3339();

    let payload = json!({
        "content": null,
        "embeds": [{
            "title": "Scraping Blocked",
            "description": "<@here>",
            "color": 15158332,
            "fields": [
                {"name": "Status", "value": format!("Scraping failed after {} attempts", max_retries)},
                {"name": "Failed Locations", "value": failed_locations.to_string()}
            ],
            "timestamp": ts
        }],
        "attachments": []
    }).to_string();

    reqwest::Client::new()
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await?
        .error_for_status()?;

    log::info!("Discord notification sent for scraping blocked after {} retries", max_retries);
    Ok(())
}
