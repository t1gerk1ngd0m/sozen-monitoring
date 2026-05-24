use anyhow::{Context, Result};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

use crate::config::Config;
use crate::model::Slot;

pub async fn send_email(ses: &aws_sdk_sesv2::Client, cfg: &Config, slots: &[Slot]) -> Result<()> {
  let subject = Content::builder()
    .data(format!("[sozen-monitor] 空き枠 {} 件検出", slots.len()))
    .charset("UTF-8")
    .build()
    .context("build subject")?;

  let body_text = Content::builder()
    .data(render_text(slots))
    .charset("UTF-8")
    .build()
    .context("build body text")?;

  let body = Body::builder().text(body_text).build();

  let message = Message::builder().subject(subject).body(body).build();

  let dest = Destination::builder().to_addresses(&cfg.email_to).build();

  ses
    .send_email()
    .from_email_address(&cfg.email_from)
    .destination(dest)
    .content(EmailContent::builder().simple(message).build())
    .send()
    .await
    .context("SES SendEmail failed")?;
  Ok(())
}

fn render_text(slots: &[Slot]) -> String {
  let mut s = String::from("以下の空き枠が検出されました:\n\n");
  for slot in slots {
    let label = slot.label.as_deref().unwrap_or("");
    s.push_str(&format!("- {} {} {}\n", slot.date, slot.time, label));
  }
  s
}
