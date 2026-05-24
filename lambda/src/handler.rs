use anyhow::Result;

use crate::availability;
use crate::config::Config;
use crate::model::Slot;
use crate::notifier;
use crate::scraper;
use crate::Deps;

pub async fn run(cfg: &Config, deps: &Deps) -> Result<()> {
  let html = scraper::fetch_html(&deps.http, &cfg.target_url).await?;
  let slots = scraper::parse_slots(&html)?;
  tracing::info!(count = slots.len(), "parsed slots");

  if !availability::has_availability(&slots) {
    tracing::info!("no availability — skip notify");
    return Ok(());
  }

  notify_all(cfg, deps, &slots).await;
  Ok(())
}

async fn notify_all(cfg: &Config, deps: &Deps, slots: &[Slot]) {
  let email_fut = notifier::email::send_email(&deps.ses, cfg, slots);
  let discord_fut = async {
    match &cfg.discord_webhook_url {
      Some(url) => notifier::webhook::post_discord(&deps.http, url, slots).await,
      None => Ok(()),
    }
  };
  let slack_fut = async {
    match &cfg.slack_webhook_url {
      Some(url) => notifier::webhook::post_slack(&deps.http, url, slots).await,
      None => Ok(()),
    }
  };

  let (e, d, s) = tokio::join!(email_fut, discord_fut, slack_fut);
  if let Err(err) = e {
    tracing::error!(error = ?err, "email notify failed");
  }
  if let Err(err) = d {
    tracing::error!(error = ?err, "discord notify failed");
  }
  if let Err(err) = s {
    tracing::error!(error = ?err, "slack notify failed");
  }
}
