//! Downloads 2026 Central League game results from NPB's official English site and writes
//! one row of standings per game day.
//!
//! Usage (from the repo root):
//!   cargo run --manifest-path tools/npb_standings/Cargo.toml -- [options]
//!
//! Options:
//!   --out <file>      CSV to write          (default data/2026_cl_standings.csv)
//!   --cache <dir>     HTML cache directory  (default data/cache/npb)
//!   --as-of <date>    ignore games after this date, YYYYMMDD
//!   --refresh         ignore the cache and download every page again

mod parse;
mod standings;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use parse::{parse_page, Game};
use standings::{compute_days, Day, TEAMS};

const BASE_URL: &str = "https://npb.jp/bis/eng/teams";
/// `04` covers March and April on NPB's site; later months have their own page.
const MONTHS: [&str; 7] = ["04", "05", "06", "07", "08", "09", "10"];
/// Pause between downloads, to be polite to NPB's server.
const REQUEST_DELAY: Duration = Duration::from_millis(1000);

struct Args {
    out: PathBuf,
    cache: PathBuf,
    as_of: Option<u32>,
    refresh: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        out: PathBuf::from("data/2026_cl_standings.csv"),
        cache: PathBuf::from("data/cache/npb"),
        as_of: None,
        refresh: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match flag.as_str() {
            "--out" => args.out = PathBuf::from(value("--out")?),
            "--cache" => args.cache = PathBuf::from(value("--cache")?),
            "--as-of" => {
                let v = value("--as-of")?;
                args.as_of = Some(v.parse().map_err(|_| format!("bad --as-of date: {v}"))?);
            }
            "--refresh" => args.refresh = true,
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(args)
}

/// Returns the page HTML from the cache, or downloads it. `None` means NPB has no such page
/// (HTTP 404), for example a month with no games yet.
fn load_page(
    agent: &ureq::Agent,
    cache: &Path,
    team: &str,
    month: &str,
    refresh: bool,
) -> Result<Option<String>, String> {
    let name = format!("calendar_{team}_{month}.html");
    let cached = cache.join(&name);
    if !refresh {
        if let Ok(html) = fs::read_to_string(&cached) {
            return Ok(Some(html));
        }
    }

    let url = format!("{BASE_URL}/{name}");
    println!("  downloading {url}");
    thread::sleep(REQUEST_DELAY);
    let mut response = agent.get(&url).call().map_err(|e| format!("{url}: {e}"))?;
    match response.status().as_u16() {
        200 => {
            let html = response
                .body_mut()
                .read_to_string()
                .map_err(|e| format!("{url}: {e}"))?;
            fs::write(&cached, &html).map_err(|e| format!("{}: {e}", cached.display()))?;
            Ok(Some(html))
        }
        404 => Ok(None),
        status => Err(format!("{url}: unexpected HTTP status {status}")),
    }
}

fn write_csv(path: &Path, days: &[Day]) -> Result<(), String> {
    let mut csv = String::from("day_index,date,rank1,rank2,rank3,rank4,rank5,rank6\n");
    for day in days {
        let ranks: Vec<String> = day.standings.iter().map(|t| t.to_string()).collect();
        csv.push_str(&format!("{},{},{}\n", day.day_index, day.date, ranks.join(",")));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(path, csv).map_err(|e| format!("{}: {e}", path.display()))
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    fs::create_dir_all(&args.cache).map_err(|e| format!("{}: {e}", args.cache.display()))?;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent("prediction-league-demo/0.1 (Solana School demo; one-off data import)")
        .build()
        .into();

    let mut games: Vec<Game> = Vec::new();
    let mut skipped = 0;
    let mut unrecognized: BTreeSet<String> = BTreeSet::new();
    let mut missing_pages = Vec::new();

    println!("Loading calendar pages for {} teams x {} months", TEAMS.len(), MONTHS.len());
    for team in TEAMS {
        let code = team.code.to_lowercase();
        for month in MONTHS {
            match load_page(&agent, &args.cache, &code, month, args.refresh)? {
                Some(html) => {
                    let page = parse_page(&html);
                    skipped += page.skipped;
                    unrecognized.extend(page.unrecognized);
                    games.extend(page.games);
                }
                None => missing_pages.push(format!("calendar_{code}_{month}.html")),
            }
        }
    }

    if !unrecognized.is_empty() {
        return Err(format!(
            "could not understand these game entries, refusing to continue: {unrecognized:?}"
        ));
    }

    let days = compute_days(&games, args.as_of);
    let Some(last) = days.last() else {
        return Err("no games found".into());
    };
    write_csv(&args.out, &days)?;

    println!();
    println!("Pages not found (404): {}", missing_pages.len());
    for page in &missing_pages {
        println!("  {page}");
    }
    println!("Game entries not played (postponed or scheduled): {skipped}");
    println!(
        "Wrote {} days ({} to {}) to {}",
        days.len(),
        days[0].date,
        last.date,
        args.out.display()
    );

    println!();
    println!("Standings after {}:", last.date);
    println!("{:<4}{:<26}{:>5}{:>5}{:>5}{:>5}  PCT", "#", "Team", "G", "W", "L", "T");
    for (rank, &team) in last.standings.iter().enumerate() {
        let rec = last.records[team as usize];
        let pct = rec.wins as f64 / (rec.wins + rec.losses).max(1) as f64;
        println!(
            "{:<4}{:<26}{:>5}{:>5}{:>5}{:>5}  {:.3}",
            rank + 1,
            TEAMS[team as usize].name,
            rec.games(),
            rec.wins,
            rec.losses,
            rec.ties,
            pct
        );
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
