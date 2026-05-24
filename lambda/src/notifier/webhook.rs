use anyhow::{Context, Result};
use serde::Serialize;

use crate::model::MenuItem;

#[derive(Serialize)]
struct DiscordPayload<'a> {
  content: &'a str,
}

#[derive(Serialize)]
struct SlackPayload<'a> {
  text: &'a str,
}

pub async fn post_discord(client: &wreq::Client, url: &str, items: &[MenuItem]) -> Result<()> {
  let text = render(items);
  client
    .post(url)
    .json(&DiscordPayload { content: &text })
    .send()
    .await
    .context("discord webhook POST")?
    .error_for_status()
    .context("discord webhook non-2xx")?;
  Ok(())
}

pub async fn post_slack(client: &wreq::Client, url: &str, items: &[MenuItem]) -> Result<()> {
  let text = render(items);
  client
    .post(url)
    .json(&SlackPayload { text: &text })
    .send()
    .await
    .context("slack webhook POST")?
    .error_for_status()
    .context("slack webhook non-2xx")?;
  Ok(())
}

fn render(items: &[MenuItem]) -> String {
  let mut s = format!("[sozen-monitor] 予約可能枠あり ({} 件)\n", items.len());
  for item in items {
    s.push_str(&format!("- {}\n", item.title));
  }
  s.push_str("https://reserva.be/sugamo401");
  s
}
