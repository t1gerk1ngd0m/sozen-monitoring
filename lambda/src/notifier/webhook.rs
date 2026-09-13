use anyhow::{Context, Result};
use serde::Serialize;

use crate::model::AvailableDay;

#[derive(Serialize)]
struct DiscordPayload<'a> {
  content: &'a str,
}

#[derive(Serialize)]
struct SlackPayload<'a> {
  text: &'a str,
}

pub async fn post_discord(
  client: &wreq::Client,
  url: &str,
  days: &[AvailableDay],
  target_url: &str,
) -> Result<()> {
  let text = render(days, target_url);
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

pub async fn post_slack(
  client: &wreq::Client,
  url: &str,
  days: &[AvailableDay],
  target_url: &str,
) -> Result<()> {
  let text = render(days, target_url);
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

fn render(days: &[AvailableDay], target_url: &str) -> String {
  let mut s = format!("[sozen-monitor] 予約可能日あり ({} 日)\n", days.len());
  for day in days {
    s.push_str(&format!("- {}\n", day.date));
  }
  s.push_str(target_url);
  s
}
