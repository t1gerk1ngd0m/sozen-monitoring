mod availability;
mod config;
mod handler;
mod model;
mod notifier;
mod scraper;

use std::time::Duration;

use anyhow::Context;
use lambda_runtime::{service_fn, Error, LambdaEvent};

use crate::config::Config;

pub struct Deps {
  pub http: wreq::Client,
  pub ses: aws_sdk_sesv2::Client,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
    )
    .json()
    .with_target(false)
    .with_current_span(false)
    .init();

  let aws_cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
  let ssm = aws_sdk_ssm::Client::new(&aws_cfg);
  let cfg = Config::load(&ssm).await.context("load config")?;

  // cf_clearance は Cookie ヘッダー直指定だと cookie jar のセッション Cookie
  // （ページ GET → AjaxSearch POST で必要）を潰すため、jar に入れる
  let jar = std::sync::Arc::new(wreq::cookie::Jar::default());
  if let Some(clearance) = &cfg.cf_clearance {
    jar.add(
      format!("cf_clearance={clearance}; Path=/").as_str(),
      cfg.target_url.as_str(),
    );
  }

  let deps = Deps {
    http: wreq::Client::builder()
      .emulation(wreq_util::Emulation::Chrome137)
      .connect_timeout(Duration::from_secs(5))
      .timeout(Duration::from_secs(15))
      .cookie_provider(jar)
      .build()
      .context("build rquest client")?,
    ses: aws_sdk_sesv2::Client::new(&aws_cfg),
  };

  // Lambda 実行プロセスのライフタイム全体で生存させる
  let cfg: &'static Config = Box::leak(Box::new(cfg));
  let deps: &'static Deps = Box::leak(Box::new(deps));

  lambda_runtime::run(service_fn(
    move |_event: LambdaEvent<serde_json::Value>| async move {
      handler::run(cfg, deps).await.map_err(Error::from)?;
      Ok::<_, Error>(serde_json::json!({ "status": "ok" }))
    },
  ))
  .await
}
