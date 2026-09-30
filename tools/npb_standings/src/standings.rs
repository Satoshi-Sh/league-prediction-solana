//! Builds the daily Central League standings from game results.
//!
//! Ranking rule:
//! - Teams are ordered by winning percentage, highest first: `wins / (wins + losses)`.
//!   Ties (draws) are not counted in the percentage, which is how NPB ranks teams.
//! - Teams with exactly the same percentage keep the order they had the day before.
//!   On the first day that starting order is the fixed team index order below.
//!
//! Interleague games count towards a team's record, as in NPB's official table.

use std::collections::{BTreeMap, HashSet};

use crate::parse::Game;

/// Central League teams in the order used by the Solana program (team index = position).
/// `code` is the abbreviation NPB uses on its calendar pages.
pub const TEAMS: [Team; 6] = [
    Team { code: "G", name: "Yomiuri Giants" },
    Team { code: "T", name: "Hanshin Tigers" },
    Team { code: "DB", name: "YOKOHAMA DeNA BAYSTARS" },
    Team { code: "C", name: "Hiroshima Toyo Carp" },
    Team { code: "D", name: "Chunichi Dragons" },
    Team { code: "S", name: "Tokyo Yakult Swallows" },
];

pub struct Team {
    pub code: &'static str,
    pub name: &'static str,
}

pub fn team_index(code: &str) -> Option<usize> {
    TEAMS.iter().position(|t| t.code == code)
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Record {
    pub wins: u32,
    pub losses: u32,
    pub ties: u32,
}

impl Record {
    pub fn games(&self) -> u32 {
        self.wins + self.losses + self.ties
    }
}

/// Standings after one game day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// 1, 2, 3, ... counting only days on which a Central League game was played.
    pub day_index: u16,
    /// YYYYMMDD
    pub date: u32,
    /// `standings[rank] = team index`, matching the on-chain `post_daily_result` argument.
    pub standings: [u8; 6],
    /// Cumulative records after this day, indexed by team index.
    pub records: [Record; 6],
}

/// Compares winning percentages exactly (no floating point): is `a` better than `b`?
/// A team with no decided games counts as 0%.
fn cmp_pct(a: &Record, b: &Record) -> std::cmp::Ordering {
    let (a_num, a_den) = (a.wins, (a.wins + a.losses).max(1));
    let (b_num, b_den) = (b.wins, (b.wins + b.losses).max(1));
    // a_num / a_den  vs  b_num / b_den
    (a_num * b_den).cmp(&(b_num * a_den))
}

/// Sorts teams by percentage, highest first. The sort is stable, so equal teams keep `previous`.
fn rank(previous: [u8; 6], records: &[Record; 6]) -> [u8; 6] {
    let mut order = previous;
    order.sort_by(|&x, &y| cmp_pct(&records[y as usize], &records[x as usize]));
    order
}

/// Computes the standings after every day that has at least one finished Central League game.
/// `as_of` (YYYYMMDD) drops later days. Games are de-duplicated by id, because each game
/// appears on the calendar pages of both teams.
pub fn compute_days(games: &[Game], as_of: Option<u32>) -> Vec<Day> {
    let mut seen = HashSet::new();
    let mut by_date: BTreeMap<u32, Vec<&Game>> = BTreeMap::new();
    for game in games {
        if as_of.is_some_and(|limit| game.date > limit) {
            continue;
        }
        if seen.insert(game.id.as_str()) {
            by_date.entry(game.date).or_default().push(game);
        }
    }

    let mut records = [Record::default(); 6];
    let mut order: [u8; 6] = [0, 1, 2, 3, 4, 5];
    let mut days = Vec::new();

    for (date, day_games) in by_date {
        let mut played_cl_game = false;

        for game in day_games {
            let a = team_index(&game.team_a);
            let b = team_index(&game.team_b);
            if a.is_none() && b.is_none() {
                continue; // Pacific League game
            }
            played_cl_game = true;

            let outcome = game.score_a.cmp(&game.score_b);
            for (team, is_a) in [(a, true), (b, false)] {
                let Some(i) = team else { continue };
                match (outcome, is_a) {
                    (std::cmp::Ordering::Equal, _) => records[i].ties += 1,
                    (std::cmp::Ordering::Greater, true) | (std::cmp::Ordering::Less, false) => {
                        records[i].wins += 1
                    }
                    _ => records[i].losses += 1,
                }
            }
        }

        if !played_cl_game {
            continue;
        }
        order = rank(order, &records);
        days.push(Day {
            day_index: days.len() as u16 + 1,
            date,
            standings: order,
            records,
        });
    }
    days
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, date: u32, a: &str, sa: u32, b: &str, sb: u32) -> Game {
        Game {
            id: id.into(),
            date,
            team_a: a.into(),
            score_a: sa,
            team_b: b.into(),
            score_b: sb,
        }
    }

    #[test]
    fn winner_moves_up_and_losers_keep_their_order() {
        // Start order: G T DB C D S. Day 1: S beats G.
        let days = compute_days(&[game("a", 20260327, "S", 3, "G", 1)], None);
        assert_eq!(days.len(), 1);
        // S is 1-0 (100%), G is 0-1, the four teams without games are 0% too.
        // S first; everyone else keeps the previous relative order (G T DB C D).
        assert_eq!(days[0].standings, [5, 0, 1, 2, 3, 4]);
        assert_eq!(days[0].records[5], Record { wins: 1, losses: 0, ties: 0 });
        assert_eq!(days[0].records[0], Record { wins: 0, losses: 1, ties: 0 });
    }

    #[test]
    fn equal_percentages_keep_previous_day_order() {
        // Day 1: T beats G. Order: T G DB C D S.
        // Day 2: G beats T. Both are now 1-1 (50%), so T stays ahead of G.
        let days = compute_days(
            &[
                game("a", 20260327, "T", 2, "G", 0),
                game("b", 20260328, "G", 5, "T", 1),
            ],
            None,
        );
        assert_eq!(days[0].standings[..2], [1, 0]);
        assert_eq!(days[1].standings[..2], [1, 0]);
    }

    #[test]
    fn ties_do_not_count_in_the_percentage() {
        // G: 1 win, 1 tie (100%). T: 1 win, 1 loss (50%).
        let days = compute_days(
            &[
                game("a", 20260327, "G", 1, "D", 0),
                game("b", 20260327, "T", 4, "S", 2),
                game("c", 20260328, "G", 2, "C", 2),
                game("d", 20260328, "D", 3, "T", 1),
            ],
            None,
        );
        let last = days.last().unwrap();
        assert_eq!(last.records[0], Record { wins: 1, losses: 0, ties: 1 });
        // G (100%) above T (50%)
        let pos = |team: usize| last.standings.iter().position(|&t| t as usize == team).unwrap();
        assert!(pos(0) < pos(1));
    }

    #[test]
    fn same_game_on_both_pages_is_counted_once() {
        let g = game("a", 20260327, "G", 3, "T", 1);
        let days = compute_days(&[g.clone(), g], None);
        assert_eq!(days[0].records[0].games(), 1);
        assert_eq!(days[0].records[1].games(), 1);
    }

    #[test]
    fn pacific_only_days_are_skipped_and_interleague_counts() {
        let days = compute_days(
            &[
                game("a", 20260327, "H", 3, "L", 1),  // Pacific only: no day row
                game("b", 20260328, "G", 4, "H", 2),  // interleague: counts for G
            ],
            None,
        );
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].day_index, 1);
        assert_eq!(days[0].date, 20260328);
        assert_eq!(days[0].records[0], Record { wins: 1, losses: 0, ties: 0 });
    }

    #[test]
    fn as_of_drops_later_days() {
        let games = [
            game("a", 20260327, "G", 3, "T", 1),
            game("b", 20260328, "G", 3, "T", 1),
        ];
        assert_eq!(compute_days(&games, Some(20260327)).len(), 1);
        assert_eq!(compute_days(&games, None).len(), 2);
    }

    #[test]
    fn standings_are_always_a_valid_permutation() {
        let days = compute_days(
            &[
                game("a", 20260327, "G", 3, "T", 1),
                game("b", 20260327, "DB", 0, "C", 2),
                game("c", 20260328, "D", 1, "S", 1),
            ],
            None,
        );
        for day in days {
            let mut sorted = day.standings;
            sorted.sort_unstable();
            assert_eq!(sorted, [0, 1, 2, 3, 4, 5]);
        }
    }
}
