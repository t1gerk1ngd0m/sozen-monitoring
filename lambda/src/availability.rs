use crate::model::MenuItem;

/// 「空きあり」と判定するロジック。
/// threshold より多い件数の menu__item があれば空きありとみなす。
/// 例: threshold=2 なら 3 件以上で true。
pub fn has_availability(items: &[MenuItem], threshold: usize) -> bool {
  items.len() > threshold
}
