use anyhow::{Context, Result};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

use crate::config::Config;
use crate::model::AvailableDay;

pub async fn send_email(
  ses: &aws_sdk_sesv2::Client,
  cfg: &Config,
  days: &[AvailableDay],
) -> Result<()> {
  let subject = Content::builder()
    .data("[sozen-monitor] そうぜん先生の予約枠あり".to_string())
    .charset("UTF-8")
    .build()
    .context("build subject")?;

  let body_text = Content::builder()
    .data(render_text(days, &cfg.target_url))
    .charset("UTF-8")
    .build()
    .context("build body text")?;

  let body = Body::builder().text(body_text).build();
  let message = Message::builder().subject(subject).body(body).build();

  for addr in &cfg.email_to {
    let dest = Destination::builder().to_addresses(addr).build();
    ses
      .send_email()
      .from_email_address(addr)
      .destination(dest)
      .content(EmailContent::builder().simple(message.clone()).build())
      .send()
      .await
      .with_context(|| format!("SES SendEmail to {addr} failed"))?;
  }
  Ok(())
}

fn render_text(days: &[AvailableDay], url: &str) -> String {
  let mut s = String::from("院長担当メニューに予約可能な日があります:\n\n");
  for day in days {
    s.push_str(&format!("- {}\n", day.date));
  }
  s.push_str(&format!("\n{url}\n"));
  s
}
