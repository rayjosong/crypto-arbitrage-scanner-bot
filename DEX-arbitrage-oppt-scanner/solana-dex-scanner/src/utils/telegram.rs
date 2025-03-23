use anyhow::Result;
use colored::*;
use reqwest::Client;
use serde_json::json;

pub async fn send_telegram_message(
    bot_token: &str,
    chat_id: &str,
    message: &str,
) -> Result<()> {
    let client = Client::new();
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        bot_token
    );

    let response = client
        .post(&url)
        .json(&json!({
            "chat_id": chat_id,
            "text": message,
            "parse_mode": "HTML"
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        println!(
            "{} Failed to send Telegram message: {}",
            "[ERROR]".bright_red(),
            response.text().await?
        );
    }

    Ok(())
} 