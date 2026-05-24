use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::model::MenuItem;

pub async fn fetch_html(
  client: &wreq::Client,
  url: &str,
  cf_clearance: Option<&str>,
) -> Result<String> {
  let mut req = client.get(url);
  if let Some(clearance) = cf_clearance {
    req = req.header("Cookie", format!("cf_clearance={clearance}"));
  }

  let res = req
    .send()
    .await
    .with_context(|| format!("GET {url} failed"))?
    .error_for_status()
    .with_context(|| format!("non-2xx response from {url}"))?;
  res.text().await.context("read response body")
}

pub fn parse_items(html: &str) -> Result<Vec<MenuItem>> {
  let doc = Html::parse_document(html);

  let item_sel = Selector::parse("ul.menu__list li.menu__item.change-color__mouseover__bg")
    .map_err(|e| anyhow::anyhow!("invalid item selector: {e:?}"))?;
  let title_sel = Selector::parse(".menu__info__title")
    .map_err(|e| anyhow::anyhow!("invalid title selector: {e:?}"))?;

  let mut items = Vec::new();
  for el in doc.select(&item_sel) {
    let title = el
      .select(&title_sel)
      .next()
      .map(|t| t.text().collect::<String>().trim().to_string())
      .unwrap_or_default();
    items.push(MenuItem { title });
  }
  Ok(items)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_real_page_fixture() {
    let html = include_str!("../tests/fixtures/sample_page.html");
    let items = parse_items(html).expect("should parse");
    assert_eq!(items.len(), 2, "現状ページの menu__item は 2 件のはず");
    for item in &items {
      assert!(!item.title.is_empty(), "title は非空");
      assert!(
        item.title.contains("鍼"),
        "タイトルに「鍼」が含まれる: {}",
        item.title
      );
    }
  }

  #[test]
  fn empty_html_returns_zero_items() {
    let items = parse_items("<html><body></body></html>").expect("should parse");
    assert!(items.is_empty());
  }
}
