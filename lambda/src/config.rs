use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
  pub target_url: String,
  pub email_from: String,
  pub email_to: String,
  pub user_agent: String,
  pub discord_webhook_url: Option<String>,
  pub slack_webhook_url: Option<String>,
  pub cf_clearance: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Webhooks {
  discord: Option<String>,
  slack: Option<String>,
  cf_clearance: Option<String>,
}

impl Config {
  pub async fn load(ssm: &aws_sdk_ssm::Client) -> Result<Self> {
    let webhook_param_name = Self::env_var("WEBHOOK_PARAM_NAME")?;
    let webhooks = Self::fetch_webhooks(ssm, &webhook_param_name).await?;

    Ok(Self {
      target_url: Self::env_var("TARGET_URL")?,
      email_from: Self::env_var("EMAIL_FROM")?,
      email_to: Self::env_var("EMAIL_TO")?,
      user_agent: Self::env_var("USER_AGENT")?,
      discord_webhook_url: webhooks.discord,
      slack_webhook_url: webhooks.slack,
      cf_clearance: webhooks.cf_clearance,
    })
  }

  fn env_var(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("env var {key} is not set"))
  }

  async fn fetch_webhooks(ssm: &aws_sdk_ssm::Client, name: &str) -> Result<Webhooks> {
    let out = ssm
      .get_parameter()
      .name(name)
      .with_decryption(true)
      .send()
      .await
      .with_context(|| format!("failed to get SSM parameter {name}"))?;

    let value = out
      .parameter
      .and_then(|p| p.value)
      .context("SSM parameter has no value")?;

    serde_json::from_str(&value).context("failed to parse webhook JSON")
  }
}
