use crate::constants::MAX_USERNAME_LEN;

/// True if `order` contains each of the 6 teams (0..6) exactly once.
pub fn is_valid_order(order: &[u8; 6]) -> bool {
    let mut seen = [false; 6];
    for &team in order {
        if team >= 6 || seen[team as usize] {
            return false;
        }
        seen[team as usize] = true;
    }
    true
}

/// 1 to MAX_USERNAME_LEN bytes, and not just whitespace. Duplicates are allowed.
pub fn is_valid_username(username: &str) -> bool {
    !username.trim().is_empty() && username.len() <= MAX_USERNAME_LEN
}
