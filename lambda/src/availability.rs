use crate::model::Slot;

/// 「空きあり」と判定するロジック。業務要件に合わせて書き換える。
///
/// 初期実装: スロットが 1 件以上あれば true。
/// 例: 特定日付に絞る、担当者ラベルで絞る、件数閾値を設けるなど。

pub fn has_availability(slots: &[Slot]) -> bool {
  !slots.is_empty()
}
