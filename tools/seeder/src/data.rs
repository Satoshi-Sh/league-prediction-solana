//! Reading the two CSV files and deriving the demo wallets. No network code here, so it is
//! easy to test.

use sha2::{Digest, Sha256};

/// One pundit's prediction. `order[rank] = team index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionRow {
    pub username: String,
    pub order: [u8; 6],
}

/// Standings after one game day. `standings[rank] = team index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayRow {
    pub day_index: u16,
    pub date: u32,
    pub standings: [u8; 6],
}

fn is_permutation(order: &[u8; 6]) -> bool {
    let mut sorted = *order;
    sorted.sort_unstable();
    sorted == [0, 1, 2, 3, 4, 5]
}

fn ranks(record: &csv::StringRecord, first: usize, line: &str) -> Result<[u8; 6], String> {
    let mut order = [0u8; 6];
    for (i, slot) in order.iter_mut().enumerate() {
        let cell = record.get(first + i).ok_or(format!("{line}: missing rank{}", i + 1))?;
        *slot = cell.trim().parse().map_err(|_| format!("{line}: bad rank {cell:?}"))?;
    }
    if !is_permutation(&order) {
        return Err(format!("{line}: ranks are not each team exactly once: {order:?}"));
    }
    Ok(order)
}

/// Columns: `username,name_ja,rank1..rank6`.
pub fn parse_predictions(text: &str) -> Result<Vec<PredictionRow>, String> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for (n, record) in reader.records().enumerate() {
        let line = format!("predictions row {}", n + 1);
        let record = record.map_err(|e| format!("{line}: {e}"))?;
        let username = record.get(0).ok_or(format!("{line}: missing username"))?.trim();
        if username.is_empty() {
            return Err(format!("{line}: empty username"));
        }
        rows.push(PredictionRow {
            username: username.to_string(),
            order: ranks(&record, 2, &line)?,
        });
    }
    Ok(rows)
}

/// Columns: `day_index,date,rank1..rank6`. Days must be 1, 2, 3, ... with no gaps.
pub fn parse_standings(text: &str) -> Result<Vec<DayRow>, String> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let mut rows: Vec<DayRow> = Vec::new();
    for (n, record) in reader.records().enumerate() {
        let line = format!("standings row {}", n + 1);
        let record = record.map_err(|e| format!("{line}: {e}"))?;
        let day_index: u16 = record
            .get(0)
            .and_then(|c| c.trim().parse().ok())
            .ok_or(format!("{line}: bad day_index"))?;
        let date: u32 = record
            .get(1)
            .and_then(|c| c.trim().parse().ok())
            .ok_or(format!("{line}: bad date"))?;
        if day_index as usize != rows.len() + 1 {
            return Err(format!("{line}: day_index {day_index} should be {}", rows.len() + 1));
        }
        if rows.last().is_some_and(|prev| date <= prev.date) {
            return Err(format!("{line}: date {date} is not after the previous day"));
        }
        rows.push(DayRow { day_index, date, standings: ranks(&record, 2, &line)? });
    }
    Ok(rows)
}

/// Deterministic 32-byte seed for a demo wallet, so re-seeding a fresh validator gives every
/// pundit the same address again. These keys hold test SOL only: never reuse them elsewhere.
pub fn wallet_seed(season_id: u64, username: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"prediction-league-demo/");
    hasher.update(season_id.to_le_bytes());
    hasher.update(b"/");
    hasher.update(username.as_bytes());
    hasher.finalize().into()
}

/// True for localhost RPC URLs. Anything else needs `--allow-remote`.
pub fn is_local_url(url: &str) -> bool {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let host = rest.split(['/', ':']).next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_predictions() {
        let text = "username,name_ja,rank1,rank2,rank3,rank4,rank5,rank6\n\
                    Iwase Hitoki,岩瀬仁紀,1,4,3,0,2,5\n";
        let rows = parse_predictions(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].username, "Iwase Hitoki");
        assert_eq!(rows[0].order, [1, 4, 3, 0, 2, 5]);
    }

    #[test]
    fn rejects_a_prediction_that_is_not_a_permutation() {
        let text = "username,name_ja,rank1,rank2,rank3,rank4,rank5,rank6\nA,x,1,1,3,0,2,5\n";
        assert!(parse_predictions(text).unwrap_err().contains("exactly once"));
    }

    #[test]
    fn parses_standings_and_checks_sequence() {
        let ok = "day_index,date,rank1,rank2,rank3,rank4,rank5,rank6\n\
                  1,20260327,0,3,5,1,2,4\n2,20260328,3,5,0,1,2,4\n";
        let rows = parse_standings(ok).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1], DayRow { day_index: 2, date: 20260328, standings: [3, 5, 0, 1, 2, 4] });

        let gap = "day_index,date,rank1,rank2,rank3,rank4,rank5,rank6\n\
                   1,20260327,0,1,2,3,4,5\n3,20260329,0,1,2,3,4,5\n";
        assert!(parse_standings(gap).unwrap_err().contains("should be 2"));

        let backwards = "day_index,date,rank1,rank2,rank3,rank4,rank5,rank6\n\
                         1,20260328,0,1,2,3,4,5\n2,20260327,0,1,2,3,4,5\n";
        assert!(parse_standings(backwards).unwrap_err().contains("not after"));
    }

    #[test]
    fn wallet_seed_is_stable_and_distinct() {
        assert_eq!(wallet_seed(2026, "Iwase Hitoki"), wallet_seed(2026, "Iwase Hitoki"));
        assert_ne!(wallet_seed(2026, "Iwase Hitoki"), wallet_seed(2026, "Satozaki Tomoya"));
        assert_ne!(wallet_seed(2026, "Iwase Hitoki"), wallet_seed(2027, "Iwase Hitoki"));
    }

    #[test]
    fn only_localhost_is_local() {
        assert!(is_local_url("http://127.0.0.1:8899"));
        assert!(is_local_url("http://localhost:8899"));
        assert!(!is_local_url("https://api.devnet.solana.com"));
        assert!(!is_local_url("https://api.mainnet-beta.solana.com"));
        assert!(!is_local_url("http://localhost.evil.example:8899"));
    }
}
