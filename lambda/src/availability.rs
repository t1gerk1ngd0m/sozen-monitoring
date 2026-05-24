use crate::model::MenuItem;

/// 「空きあり」と判定するロジック。
/// 現状ページは menu__item が 2 件。3 件以上に増えたら空きありとみなす。
pub fn has_availability(items: &[MenuItem]) -> bool {
  items.len() > 2
}
