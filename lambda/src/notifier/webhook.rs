use anyhow::{Context, Result};
use serde::Serialize;

use crate::model::Slot;

#[derive(Serialize)]
struct DiscordPayload<'a> {
  content: &'a str,
}

#[derive(Serialize)]
struct SlackPayload<'a> {
  text: &'a str,
}

pub async fn post_discord(client: &reqwest::Client, url: &str, slots: &[Slot]) -> Result<()> {
  let text = render(slots);
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

pub async fn post_slack(client: &reqwest::Client, url: &str, slots: &[Slot]) -> Result<()> {
  let text = render(slots);
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

fn render(slots: &[Slot]) -> String {
  let mut s = format!("[sozen-monitor] 空き枠 {} 件検出\n", slots.len());
  for slot in slots {
    let label = slot.label.as_deref().unwrap_or("");
    s.push_str(&format!("- {} {} {}\n", slot.date, slot.time, label));
  }
  s
}
