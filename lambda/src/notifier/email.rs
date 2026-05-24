use anyhow::{Context, Result};
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

use crate::config::Config;
use crate::model::MenuItem;

pub async fn send_email(
  ses: &aws_sdk_sesv2::Client,
  cfg: &Config,
  items: &[MenuItem],
) -> Result<()> {
  let subject = Content::builder()
    .data(format!("[sozen-monitor] そうぜん先生の予約枠あり"))
    .charset("UTF-8")
    .build()
    .context("build subject")?;

  let body_text = Content::builder()
    .data(render_text(items))
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

fn render_text(items: &[MenuItem]) -> String {
  let mut s = String::from("以下のメニューが表示されています:\n\n");
  for item in items {
    s.push_str(&format!("- {}\n", item.title));
  }
  s.push_str("\nhttps://reserva.be/sugamo401\n");
  s
}
