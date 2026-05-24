#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let url = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "https://reserva.be/sugamo401".to_string());
  let cookie = std::env::var("CF_CLEARANCE").ok();

  let client = wreq::Client::builder()
    .emulation(wreq_util::Emulation::Chrome137)
    .cookie_store(true)
    .build()?;

  let mut req = client.get(&url);
  if let Some(c) = &cookie {
    req = req.header("Cookie", format!("cf_clearance={c}"));
  }

  let res = req.send().await?;
  let status = res.status();
  let body = res.text().await?;

  std::fs::write("/tmp/fetch_test.html", &body)?;
  eprintln!("Status: {status}");
  eprintln!("Size: {} bytes", body.len());
  eprintln!("Saved to /tmp/fetch_test.html");

  if body.contains("Just a moment") || body.contains("cf_chl_opt") {
    eprintln!("❌ STILL BLOCKED by Cloudflare challenge");
  } else if body.contains("menu__list") {
    eprintln!("✓ Got real page (found menu__list)");
  } else {
    eprintln!("⚠ Unknown response — inspect /tmp/fetch_test.html");
  }
  Ok(())
}
