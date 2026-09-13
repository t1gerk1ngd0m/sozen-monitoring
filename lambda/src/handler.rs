use anyhow::Result;

use crate::availability;
use crate::config::Config;
use crate::model::AvailableDay;
use crate::notifier;
use crate::scraper;
use crate::Deps;

/// 初月に枠が無い場合に追加で確認する月数
const EXTRA_MONTHS_TO_CHECK: usize = 2;

pub async fn run(cfg: &Config, deps: &Deps) -> Result<()> {
  let html = scraper::fetch_html(&deps.http, &cfg.target_url).await?;
  let params = scraper::parse_reserve_params(&html)?;
  tracing::info!(?params, "parsed reserve params");

  let base_url = origin_of(&cfg.target_url);
  let days = collect_available_days(deps, &base_url, cfg, &params).await?;

  if days.is_empty() {
    tracing::info!("no availability — skip notify");
    return Ok(());
  }

  tracing::info!(count = days.len(), "available days found");
  notify_all(cfg, deps, &days).await;
  Ok(())
}

/// 初月（サーバーが選ぶ最初の予約対象月）から順に予約可能日を集める。
/// 月が進まなくなったら（予約受付期間の終端）打ち切る。
async fn collect_available_days(
  deps: &Deps,
  base_url: &str,
  cfg: &Config,
  params: &crate::model::ReserveParams,
) -> Result<Vec<AvailableDay>> {
  let mut days = Vec::new();
  let mut year = String::new();
  let mut month = String::new();
  let mut mode = "";

  for _ in 0..=EXTRA_MONTHS_TO_CHECK {
    let res = scraper::fetch_calendar(
      &deps.http,
      base_url,
      &cfg.target_url,
      params,
      &year,
      &month,
      mode,
    )
    .await?;

    tracing::info!(
      year = %res.reserve_year,
      month = %res.reserve_month,
      enabled = res.enabled_num(),
      status = res.status_num(),
      "calendar fetched"
    );

    // JS 側は errorno=1001 をパラメータ不正としてホームへ戻す。
    // 黙って「枠なし」扱いにすると永久に通知が止まるため、エラーで落とす
    if res.errorno_num() == 1001 {
      anyhow::bail!("AjaxSearch returned errorno=1001 (invalid params?)");
    }

    let month_days = availability::selectable_days(&res.data);
    if !month_days.is_empty() && res.enabled_num() != 1 {
      tracing::warn!(
        enabled = res.enabled_num(),
        days = month_days.len(),
        "selectable days found but enabled != 1 — check judgement logic"
      );
    }
    days.extend(month_days);

    // サーバーが月を進めなかった＝これ以上先の月は無い
    if res.reserve_year == year && res.reserve_month == month {
      break;
    }
    year = res.reserve_year;
    month = res.reserve_month;
    mode = "reserve_next";
  }

  Ok(days)
}

fn origin_of(url: &str) -> String {
  // "https://host/path..." → "https://host"
  let mut parts = url.splitn(4, '/');
  let scheme = parts.next().unwrap_or_default();
  let _ = parts.next();
  let host = parts.next().unwrap_or_default();
  format!("{scheme}//{host}")
}

async fn notify_all(cfg: &Config, deps: &Deps, days: &[AvailableDay]) {
  let email_fut = notifier::email::send_email(&deps.ses, cfg, days);
  let discord_fut = async {
    match &cfg.discord_webhook_url {
      Some(url) => notifier::webhook::post_discord(&deps.http, url, days, &cfg.target_url).await,
      None => Ok(()),
    }
  };
  let slack_fut = async {
    match &cfg.slack_webhook_url {
      Some(url) => notifier::webhook::post_slack(&deps.http, url, days, &cfg.target_url).await,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn origin_of_extracts_scheme_and_host() {
    assert_eq!(
      origin_of("https://reserva.be/sugamo401/reserve?mode=x"),
      "https://reserva.be"
    );
  }
}
