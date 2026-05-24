use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::model::Slot;

pub async fn fetch_html(client: &reqwest::Client, url: &str) -> Result<String> {
  let res = client
    .get(url)
    .send()
    .await
    .with_context(|| format!("GET {url} failed"))?
    .error_for_status()
    .with_context(|| format!("non-2xx response from {url}"))?;

  res.text().await.context("failed to read response body")
}

pub fn parse_slots(html: &str) -> Result<Vec<Slot>> {
  let doc = Html::parse_document(html);

  // TODO: 対象サイトの HTML 構造に合わせて selector を書き換える
  // 想定: <div class="slot" data-date="2026-05-25" data-time="14:00">担当者名</div>
  let slot_sel =
    Selector::parse("div.slot").map_err(|e| anyhow::anyhow!("invalid selector: {e}"))?;

  let mut slots = Vec::new();
  for el in doc.select(&slot_sel) {
    let date = el
      .value()
      .attr("data-date")
      .unwrap_or("")
      .trim()
      .to_string();
    let time = el
      .value()
      .attr("data-time")
      .unwrap_or("")
      .trim()
      .to_string();
    let label = el.value().attr("data-label").map(|s| s.to_string());

    slots.push(Slot { date, time, label });
  }

  Ok(slots)
}

#[cfg(test)]
mod tests {
  use super::*;

  const SAMPLE_HTML: &str = r#"
  <!DOCTYPE html>
  <html><body>
    <div class="slot" data-date="2026-05-25" 
  data-time="14:00">院長</div>
    <div class="slot" data-date="2026-05-25" 
  data-time="15:30"></div>
    <div class="slot" data-date="2026-05-26" 
  data-time="10:00">副院長</div>
    <div class="other">ノイズ</div>
    <div class="slot">日付欠落で無視されるはず</div>
  </body></html>
  "#;

  #[test]
  fn parses_three_slots() {
    let slots = parse_slots(SAMPLE_HTML).expect(
      "should 
  parse",
    );
    assert_eq!(
      slots.len(),
      3,
      "div.slot with date+time が 3 
  件"
    );

    assert_eq!(slots[0].date, "2026-05-25");
    assert_eq!(slots[0].time, "14:00");
    assert_eq!(slots[0].label.as_deref(), Some("院長"));

    assert_eq!(slots[1].label, None, "空テキストは None");

    assert_eq!(slots[2].date, "2026-05-26");
    assert_eq!(slots[2].label.as_deref(), Some("副院長"));
  }

  #[test]
  fn ignores_unrelated_html() {
    let html = "<html><body><p>nothing 
  here</p></body></html>";
    let slots = parse_slots(html).expect("should parse");
    assert!(slots.is_empty());
  }
}
