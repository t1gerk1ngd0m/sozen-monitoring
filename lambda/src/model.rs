use serde::{Deserialize, Serialize};

/// 予約ページから抽出する AjaxSearch 用パラメータ
#[derive(Debug, Clone)]
pub struct ReserveParams {
  pub bus_cd: String,
  pub evt_no: String,
  pub stf_cd: String,
  pub srv_time: String,
  pub bus_reserve_flag: String,
  pub bus_interval: String,
}

/// AjaxSearch (cmd=divmenu3_calendar) のレスポンス
#[derive(Debug, Deserialize)]
pub struct CalendarResponse {
  /// サーバー側は数値/文字列どちらも返しうる（JS も Number() で正規化している）
  #[serde(default)]
  pub enabled: serde_json::Value,
  #[serde(default)]
  pub status: serde_json::Value,
  #[serde(default)]
  pub errorno: serde_json::Value,
  #[serde(rename = "reserveYear", default)]
  pub reserve_year: String,
  #[serde(rename = "reserveMonth", default)]
  pub reserve_month: String,
  #[serde(default)]
  pub data: CalendarData,
}

#[derive(Debug, Default, Deserialize)]
pub struct CalendarData {
  /// 週ごとの日セル。月初・月末の空セルは [] で埋まる
  #[serde(rename = "weeklyCalendar", default)]
  pub weekly_calendar: Vec<Vec<serde_json::Value>>,
}

/// カレンダーの1日分のセル
#[derive(Debug, Clone, Deserialize)]
pub struct DayCell {
  pub reserve_date: String,
  #[serde(rename = "isSelectable", default)]
  pub is_selectable: bool,
  #[serde(default)]
  pub is_full: i64,
}

/// 予約可能な日
#[derive(Debug, Clone, Serialize)]
pub struct AvailableDay {
  pub date: String,
}

impl CalendarResponse {
  pub fn enabled_num(&self) -> i64 {
    coerce_num(&self.enabled)
  }

  pub fn status_num(&self) -> i64 {
    coerce_num(&self.status)
  }

  pub fn errorno_num(&self) -> i64 {
    coerce_num(&self.errorno)
  }
}

fn coerce_num(v: &serde_json::Value) -> i64 {
  match v {
    serde_json::Value::Number(n) => n.as_i64().unwrap_or(-1),
    serde_json::Value::String(s) => s.parse().unwrap_or(-1),
    _ => -1,
  }
}
