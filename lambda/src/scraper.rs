use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::model::{CalendarResponse, ReserveParams};

pub async fn fetch_html(client: &wreq::Client, url: &str) -> Result<String> {
  let res = client
    .get(url)
    .send()
    .await
    .with_context(|| format!("GET {url} failed"))?
    .error_for_status()
    .with_context(|| format!("non-2xx response from {url}"))?;
  res.text().await.context("read response body")
}

/// 予約ページの hidden input / ReserveSession から AjaxSearch 用パラメータを抽出する
pub fn parse_reserve_params(html: &str) -> Result<ReserveParams> {
  let doc = Html::parse_document(html);

  let hidden = |id: &str| -> Result<String> {
    let sel = Selector::parse(&format!("input#{id}"))
      .map_err(|e| anyhow::anyhow!("invalid selector for {id}: {e:?}"))?;
    doc
      .select(&sel)
      .next()
      .and_then(|el| el.value().attr("value"))
      .map(|v| v.to_string())
      .with_context(|| format!("hidden input #{id} not found"))
  };

  // ReserveSession は <script> 内の JS オブジェクトなので文字列検索で取る
  let session = |key: &str| -> Result<String> {
    let marker = format!("'{key}'");
    let line = html
      .lines()
      .find(|l| l.contains(&marker))
      .with_context(|| format!("ReserveSession key {key} not found"))?;
    let after_key = line
      .splitn(3, '\'')
      .nth(2)
      .with_context(|| format!("malformed ReserveSession line for {key}"))?;
    after_key
      .split('\'')
      .nth(1)
      .map(|v| v.to_string())
      .with_context(|| format!("no value for ReserveSession key {key}"))
  };

  Ok(ReserveParams {
    bus_cd: hidden("reserve_bus_cd")?,
    evt_no: hidden("reserve_evt_no")?,
    stf_cd: hidden("reserve_stf_cd")?,
    srv_time: hidden("reserve_srv_time")?,
    bus_reserve_flag: session("bus_reserve_flag")?,
    bus_interval: session("bus_interval")?,
  })
}

/// AjaxSearch にカレンダー取得を投げる。
/// 初回は year/month/mode を空にするとサーバーが最初の予約対象月を返す。
/// 以降は mode="reserve_next" + 直前に返ってきた年月を渡すと翌月を返す。
pub async fn fetch_calendar(
  client: &wreq::Client,
  base_url: &str,
  referer: &str,
  params: &ReserveParams,
  year: &str,
  month: &str,
  mode: &str,
) -> Result<CalendarResponse> {
  let form = [
    ("cmd", "divmenu3_calendar"),
    ("mode", mode),
    ("reserve_bus_cd", &params.bus_cd),
    ("reserve_year", year),
    ("reserve_month", month),
    ("reserve_date", &format!("{year}{month}01")),
    ("reserve_evt_no", &params.evt_no),
    ("reserve_stf_cd", &params.stf_cd),
    ("bus_reserve_flag", &params.bus_reserve_flag),
    ("rsd_group_people", "1"),
    ("reserve_srv_time", &params.srv_time),
    ("bus_interval", &params.bus_interval),
    ("is_front", "true"),
  ];
  let body = form
    .iter()
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>()
    .join("&");

  let url = format!("{base_url}/AjaxSearch");
  let res = client
    .post(&url)
    .header("Referer", referer)
    .header("X-Requested-With", "XMLHttpRequest")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await
    .with_context(|| format!("POST {url} failed"))?
    .error_for_status()
    .with_context(|| format!("non-2xx response from {url}"))?;

  let text = res.text().await.context("read AjaxSearch body")?;
  serde_json::from_str(&text).with_context(|| {
    format!(
      "AjaxSearch response is not expected JSON: {}",
      &text[..text.len().min(300)]
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_reserve_params_from_fixture() {
    let html = include_str!("../tests/fixtures/reserve_page.html");
    let p = parse_reserve_params(html).expect("should parse");
    assert_eq!(p.bus_cd, "277");
    assert_eq!(p.evt_no, "739");
    assert_eq!(p.stf_cd, "");
    assert_eq!(p.srv_time, "15");
    assert_eq!(p.bus_reserve_flag, "3");
    assert_eq!(p.bus_interval, "15");
  }

  #[test]
  fn parses_calendar_response_fixture() {
    let json = include_str!("../tests/fixtures/ajax_calendar.json");
    let res: CalendarResponse = serde_json::from_str(json).expect("should parse");
    assert_eq!(res.enabled_num(), 0);
    assert_eq!(res.reserve_year, "2026");
    assert_eq!(res.reserve_month, "09");
    assert!(!res.data.weekly_calendar.is_empty());
  }
}
