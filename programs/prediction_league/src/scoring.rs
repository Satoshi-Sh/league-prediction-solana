//! Scoring for a predicted league order against actual standings.
//!
//! Both `order` and `standings` have the form `[team index; 6]` listed by rank,
//! so `order[0]` is the team predicted to finish first.
//!
//! The score is an accuracy percentage from 0 to 100:
//!
//! 1. `distance` = sum over all teams of |predicted_rank - actual_rank|
//! 2. `score = round(100 * (MAX_DISTANCE - distance) / MAX_DISTANCE)`
//!
//! `MAX_DISTANCE` is the largest distance possible (5+3+1+1+3+5 = 18, e.g. a fully reversed
//! order). So 100 is a perfect prediction and 0 is the largest possible error. The full
//! reversal is not the only order with distance 18: any order that swaps the top three and
//! bottom three teams also scores 0 (36 orders in total).
//! Because the distance between two permutations is always even, there are 10 possible
//! scores: 100, 89, 78, 67, 56, 44, 33, 22, 11 and 0.
//!
//! The dashboard mirrors this rule in TypeScript, so keep the two in sync.

pub const TEAM_COUNT: usize = 6;
pub const MAX_SCORE: u8 = 100;
/// Largest possible total distance for `TEAM_COUNT` teams (a fully reversed order).
pub const MAX_DISTANCE: u32 = (TEAM_COUNT * TEAM_COUNT / 2) as u32;

/// Sum of how far each team is from its actual rank.
/// Both arguments must be valid permutations of 0..6 (checked by the instructions).
pub fn total_distance(order: &[u8; TEAM_COUNT], standings: &[u8; TEAM_COUNT]) -> u32 {
    // actual_rank[team] = position of that team in the standings
    let mut actual_rank = [0u8; TEAM_COUNT];
    for (rank, &team) in standings.iter().enumerate() {
        actual_rank[team as usize] = rank as u8;
    }

    order
        .iter()
        .enumerate()
        .map(|(predicted_rank, &team)| {
            (predicted_rank as u8).abs_diff(actual_rank[team as usize]) as u32
        })
        .sum()
}

/// Accuracy score from 0 to `MAX_SCORE`.
pub fn score_for(order: &[u8; TEAM_COUNT], standings: &[u8; TEAM_COUNT]) -> u8 {
    let distance = total_distance(order, standings);
    // Round to the nearest integer using integer math (adding half the divisor).
    let score = (MAX_SCORE as u32 * (MAX_DISTANCE - distance) + MAX_DISTANCE / 2) / MAX_DISTANCE;
    score as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_prediction_gets_100() {
        let standings = [2, 0, 1, 5, 4, 3];
        assert_eq!(score_for(&standings, &standings), 100);
    }

    #[test]
    fn fully_reversed_order_gets_0() {
        let standings = [0, 1, 2, 3, 4, 5];
        let reversed = [5, 4, 3, 2, 1, 0];
        assert_eq!(total_distance(&reversed, &standings), MAX_DISTANCE);
        assert_eq!(MAX_DISTANCE, 18);
        assert_eq!(score_for(&reversed, &standings), 0);
    }

    #[test]
    fn one_neighbour_swap_gets_89() {
        // Two teams one place off: distance 2 -> 100 * 16 / 18 = 88.9 -> 89
        let standings = [0, 1, 2, 3, 4, 5];
        let swapped = [1, 0, 2, 3, 4, 5];
        assert_eq!(total_distance(&swapped, &standings), 2);
        assert_eq!(score_for(&swapped, &standings), 89);
    }

    #[test]
    fn distance_is_measured_by_team_not_by_slot() {
        // Team 0 predicted last but actually first (distance 5), teams 1..5 each one place
        // off (distance 1 each): total distance 10.
        let standings = [0, 1, 2, 3, 4, 5];
        let order = [1, 2, 3, 4, 5, 0];
        assert_eq!(total_distance(&order, &standings), 10);
        // 100 * (18 - 10) / 18 = 44.4 -> 44
        assert_eq!(score_for(&order, &standings), 44);
    }

    #[test]
    fn score_is_symmetric_in_the_two_orders() {
        let a = [3, 1, 4, 0, 5, 2];
        let b = [0, 2, 1, 5, 3, 4];
        assert_eq!(score_for(&a, &b), score_for(&b, &a));
    }

    #[test]
    fn exhaustive_over_all_orders() {
        let mut perms = Vec::new();
        permute(&mut [0, 1, 2, 3, 4, 5], 0, &mut perms);
        assert_eq!(perms.len(), 720);

        // Every pair of orders stays in range. A distance above MAX_DISTANCE would
        // underflow in score_for and panic here.
        for order in &perms {
            for standings in &perms {
                assert!(total_distance(order, standings) <= MAX_DISTANCE);
                assert!(score_for(order, standings) <= MAX_SCORE);
            }
        }

        // Against any fixed standings there is exactly one perfect prediction. The worst score
        // (0) is shared by 36 orders: every order that puts the actual top three in the bottom
        // three places and vice versa (3! * 3!), of which the full reversal is one.
        // Only the 10 documented score values can occur.
        let standings = [0, 1, 2, 3, 4, 5];
        let scores: Vec<u8> = perms.iter().map(|o| score_for(o, &standings)).collect();
        assert_eq!(scores.iter().filter(|&&s| s == 100).count(), 1);
        assert_eq!(scores.iter().filter(|&&s| s == 0).count(), 36);

        let mut distinct = scores.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct, vec![0, 11, 22, 33, 44, 56, 67, 78, 89, 100]);
    }

    fn permute(items: &mut [u8; 6], start: usize, out: &mut Vec<[u8; 6]>) {
        if start == items.len() {
            out.push(*items);
            return;
        }
        for i in start..items.len() {
            items.swap(start, i);
            permute(items, start + 1, out);
            items.swap(start, i);
        }
    }
}
