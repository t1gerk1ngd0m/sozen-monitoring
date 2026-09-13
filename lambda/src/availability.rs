use crate::model::{AvailableDay, CalendarData, DayCell};

/// カレンダーから予約可能な日を抽出する。
/// isSelectable かつ満枠でない日を「予約枠あり」とみなす
/// （is_full=1 はキャンセル待ちのみの可能性があるため除外）。
pub fn selectable_days(data: &CalendarData) -> Vec<AvailableDay> {
  data
    .weekly_calendar
    .iter()
    .flatten()
    .filter_map(|cell| serde_json::from_value::<DayCell>(cell.clone()).ok())
    .filter(|d| d.is_selectable && d.is_full == 0)
    .map(|d| AvailableDay {
      date: d.reserve_date,
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::CalendarResponse;

  #[test]
  fn no_selectable_days_in_closed_month() {
    let json = include_str!("../tests/fixtures/ajax_calendar.json");
    let res: CalendarResponse = serde_json::from_str(json).expect("should parse");
    assert!(selectable_days(&res.data).is_empty());
  }

  #[test]
  fn detects_selectable_day() {
    let json = r#"{
      "weeklyCalendar": [[
        [],
        {"reserve_date": "2026-09-20", "isSelectable": true, "is_full": 0},
        {"reserve_date": "2026-09-21", "isSelectable": true, "is_full": 1},
        {"reserve_date": "2026-09-22", "isSelectable": false, "is_full": 0}
      ]]
    }"#;
    let data: CalendarData = serde_json::from_str(json).expect("should parse");
    let days = selectable_days(&data);
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].date, "2026-09-20");
  }
}
