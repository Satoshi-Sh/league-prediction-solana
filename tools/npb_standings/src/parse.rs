//! Extracts game results from an NPB "team calendar" page such as
//! https://npb.jp/bis/eng/teams/calendar_g_04.html
//!
//! Each game is a link like
//! `<a href="/bis/eng/2026/games/s2026032701085.html">G 3 - 1 T</a>`
//! The game id in the URL starts with the date (`s` + YYYYMMDD + game number).

use regex::Regex;

/// A finished game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// Unique id from the URL, e.g. `s2026032701085`. The same game appears on both teams' pages.
    pub id: String,
    /// YYYYMMDD
    pub date: u32,
    pub team_a: String,
    pub score_a: u32,
    pub team_b: String,
    pub score_b: u32,
}

#[derive(Debug, Default)]
pub struct PageResult {
    pub games: Vec<Game>,
    /// Games listed but not played: postponed or cancelled (`* - *`), or still scheduled.
    pub skipped: usize,
    /// Link texts we did not understand. Should be empty; the caller reports them.
    pub unrecognized: Vec<String>,
}

pub fn parse_page(html: &str) -> PageResult {
    let link = Regex::new(r#"href="/bis/eng/\d{4}/games/(s(\d{8})\d+)\.html">([^<]+)</a>"#).unwrap();
    let played = Regex::new(r"^([A-Za-z]+) (\d+) - (\d+) ([A-Za-z]+)$").unwrap();
    let postponed = Regex::new(r"^[A-Za-z]+ \* - \* [A-Za-z]+$").unwrap();
    let scheduled = Regex::new(r"^[A-Za-z]+ - [A-Za-z]+( \d{1,2}:\d{2})?$").unwrap();

    let mut result = PageResult::default();
    for cap in link.captures_iter(html) {
        let id = &cap[1];
        let date: u32 = cap[2].parse().unwrap();
        let text = cap[3].trim();

        if let Some(m) = played.captures(text) {
            result.games.push(Game {
                id: id.to_string(),
                date,
                team_a: m[1].to_string(),
                score_a: m[2].parse().unwrap(),
                team_b: m[4].to_string(),
                score_b: m[3].parse().unwrap(),
            });
        } else if postponed.is_match(text) || scheduled.is_match(text) {
            result.skipped += 1;
        } else {
            result.unrecognized.push(text.to_string());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        <div class="tescore"><a href="/bis/eng/2026/games/s2026032701085.html">G 3 - 1 T</a></div>
        <div class="tescore"><a href="/bis/eng/2026/games/s2026032801088.html">G 0 - 2 T</a></div>
        <div class="tescore"><a href="/bis/eng/2026/games/s2026040101098.html">D 5 - 5 G</a></div>
        <div class="tescore"><a href="/bis/eng/2026/games/s2026090801200.html">T * - * C</a></div>
        <div class="tescore"><a href="/bis/eng/2026/games/s2026100101300.html">G - S 18:00</a></div>
        <div class="tescore"><a href="/bis/eng/2026/games/s2026100201301.html">Something odd</a></div>
    "#;

    #[test]
    fn parses_played_postponed_and_scheduled_games() {
        let r = parse_page(SAMPLE);
        assert_eq!(r.games.len(), 3);
        assert_eq!(r.skipped, 2);
        assert_eq!(r.unrecognized, vec!["Something odd".to_string()]);

        assert_eq!(
            r.games[0],
            Game {
                id: "s2026032701085".into(),
                date: 20260327,
                team_a: "G".into(),
                score_a: 3,
                team_b: "T".into(),
                score_b: 1,
            }
        );
        // A tie keeps both scores equal.
        assert_eq!(r.games[2].score_a, r.games[2].score_b);
    }

    #[test]
    fn ignores_unrelated_links() {
        let r = parse_page(r#"<a href="/bis/eng/teams/calendar_g_04.html">G</a>"#);
        assert!(r.games.is_empty() && r.skipped == 0 && r.unrecognized.is_empty());
    }
}
