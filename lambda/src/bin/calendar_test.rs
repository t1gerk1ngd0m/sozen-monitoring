// AjaxSearch カレンダー取得の実験用バイナリ
#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let url = std::env::args().nth(1).unwrap_or_else(|| {
    "https://reserva.be/sugamo401/reserve?mode=service_staff&search_evt_no=b3eJwzN7YEAAFHAKQ"
      .to_string()
  });

  // 本番 (main.rs) と同じ cookie_provider(Jar) 構成で検証する
  let jar = std::sync::Arc::new(wreq::cookie::Jar::default());
  if let Ok(clearance) = std::env::var("CF_CLEARANCE") {
    jar.add(
      format!("cf_clearance={clearance}; Path=/").as_str(),
      url.as_str(),
    );
  }
  let client = wreq::Client::builder()
    .emulation(wreq_util::Emulation::Chrome137)
    .cookie_provider(jar)
    .build()?;

  let res = client.get(&url).send().await?;
  eprintln!("page status: {}", res.status());
  let html = res.text().await?;

  let get_hidden = |id: &str| -> Option<String> {
    let marker = format!("id=\"{id}\"");
    let line = html.lines().find(|l| l.contains(&marker))?;
    let v = line.split("value=\"").nth(1)?;
    Some(v.split('"').next()?.to_string())
  };
  let get_session = |key: &str| -> Option<String> {
    let marker = format!("'{key}'");
    let line = html.lines().find(|l| l.contains(&marker))?;
    let v = line.splitn(3, '\'').nth(2)?; // after key
    let v = v.split('\'').nth(1)?;
    Some(v.to_string())
  };

  let bus_cd = get_hidden("reserve_bus_cd").unwrap_or_default();
  let evt_no = get_hidden("reserve_evt_no").unwrap_or_default();
  let stf_cd = get_hidden("reserve_stf_cd").unwrap_or_default();
  let srv_time = get_hidden("reserve_srv_time").unwrap_or_default();
  let bus_reserve_flag = get_session("bus_reserve_flag").unwrap_or_default();
  let bus_interval = get_session("bus_interval").unwrap_or_default();

  eprintln!("bus_cd={bus_cd} evt_no={evt_no} stf_cd={stf_cd} srv_time={srv_time} bus_reserve_flag={bus_reserve_flag} bus_interval={bus_interval}");

  // 空のままだとサーバーが最初の予約対象月を返す（実ページの初回挙動と同じ）
  let year = std::env::args().nth(2).unwrap_or_default();
  let month = std::env::args().nth(3).unwrap_or_default();

  let form = [
    ("cmd", "divmenu3_calendar".to_string()),
    ("mode", std::env::args().nth(4).unwrap_or_default()),
    ("reserve_bus_cd", bus_cd),
    ("reserve_year", year.clone()),
    ("reserve_month", month.clone()),
    ("reserve_date", format!("{year}{month}01")),
    ("reserve_evt_no", evt_no),
    ("reserve_stf_cd", stf_cd),
    ("bus_reserve_flag", bus_reserve_flag),
    ("rsd_group_people", "1".to_string()),
    ("reserve_srv_time", srv_time),
    ("bus_interval", bus_interval),
    ("is_front", "true".to_string()),
  ];

  let body = form
    .iter()
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>()
    .join("&");

  let res = client
    .post("https://reserva.be/AjaxSearch")
    .header("Referer", &url)
    .header("X-Requested-With", "XMLHttpRequest")
    .header("Content-Type", "application/x-www-form-urlencoded")
    .body(body)
    .send()
    .await?;
  eprintln!("ajax status: {}", res.status());
  let body = res.text().await?;
  std::fs::write("/tmp/ajax_result.json", &body)?;
  eprintln!("size: {} bytes, saved to /tmp/ajax_result.json", body.len());

  if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
    eprintln!("enabled={:?} status={:?} showOverlay={:?} reserveYear={:?} reserveMonth={:?}",
      json["enabled"], json["status"], json["showOverlay"], json["reserveYear"], json["reserveMonth"]);
    if let Some(h) = json["html"].as_str() {
      eprintln!("html length: {}", h.len());
    }
  } else {
    eprintln!("not JSON: {}", &body[..body.len().min(500)]);
  }
  Ok(())
}
