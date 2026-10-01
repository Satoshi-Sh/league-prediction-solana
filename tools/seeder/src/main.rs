//! Loads the 2026 demo season into a validator: the season, one prediction per pundit, and the
//! daily standings. Safe to re-run: it only adds what is missing, so after you refresh
//! `data/2026_cl_standings.csv` it posts just the new days.
//!
//! Start a local validator first (see the README printed by `--help`), then from the repo root:
//!   cargo run --manifest-path tools/seeder/Cargo.toml
//!   cargo run --manifest-path tools/seeder/Cargo.toml -- --finalize   # when the season is over

mod data;

use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anchor_lang::{
    prelude::Pubkey,
    solana_program::{instruction::Instruction, system_instruction, system_program},
    AccountDeserialize, Discriminator, InstructionData, ToAccountMetas,
};
use data::{DayRow, PredictionRow};
use prediction_league::{scoring::score_for, DailyResult, Prediction, Season};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::{read_keypair_file, write_keypair_file, Keypair};
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::{
    config::RpcProgramAccountsConfig,
    filter::{Memcmp, RpcFilterType},
};
use solana_signer::Signer;
use solana_transaction::Transaction;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
/// Every pundit wallet gets this much to pay for its prediction account.
const PREDICTOR_FUNDING: u64 = LAMPORTS_PER_SOL / 100;
const TEAM_NAMES: [&str; 6] = ["Yomiuri", "Hanshin", "DeNA", "Hiroshima", "Chunichi", "Yakult"];
const HELP: &str = "\
Loads the 2026 demo season into a validator. Safe to re-run.

Start a local validator first, from the repo root. `anchor build` produces an SBPF v3
program, which the stock solana-test-validator 3.1.x refuses to run (\"Program is not
deployed\"), so build an SBPF v1 copy for it:
  (cd programs/prediction_league && cargo build-sbf --arch v1 --sbf-out-dir ../../target/deploy-v1)
  solana-test-validator --reset --ledger /tmp/pl-ledger --bpf-program \\
      9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP target/deploy-v1/prediction_league.so

Options:
  --rpc <url>              default http://127.0.0.1:8899 (other hosts need --allow-remote)
  --allow-remote           allow a non-local RPC. This writes real people's names on-chain.
  --admin-keypair <file>   default data/demo-admin.json (created if missing; test key only)
  --standings <file>       default data/2026_cl_standings.csv
  --predictions <file>     default data/2026_cl_predictions.csv
  --season-id <n>          default 2026
  --deadline-minutes <n>   prediction window when creating the season, default 30. Only used when the
                           season is created. For a demo where people submit predictions on the
                           dashboard, use something long, e.g. 525600 (one year)
  --finalize               end the season and write every final score (irreversible)
";

struct Args {
    rpc: String,
    allow_remote: bool,
    admin: PathBuf,
    standings: PathBuf,
    predictions: PathBuf,
    season_id: u64,
    deadline_minutes: i64,
    finalize: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        rpc: "http://127.0.0.1:8899".into(),
        allow_remote: false,
        admin: "data/demo-admin.json".into(),
        standings: "data/2026_cl_standings.csv".into(),
        predictions: "data/2026_cl_predictions.csv".into(),
        season_id: 2026,
        deadline_minutes: 30,
        finalize: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match flag.as_str() {
            "--rpc" => args.rpc = value("--rpc")?,
            "--allow-remote" => args.allow_remote = true,
            "--admin-keypair" => args.admin = value("--admin-keypair")?.into(),
            "--standings" => args.standings = value("--standings")?.into(),
            "--predictions" => args.predictions = value("--predictions")?.into(),
            "--season-id" => {
                args.season_id = value("--season-id")?.parse().map_err(|_| "bad --season-id")?
            }
            "--deadline-minutes" => {
                args.deadline_minutes =
                    value("--deadline-minutes")?.parse().map_err(|_| "bad --deadline-minutes")?
            }
            "--finalize" => args.finalize = true,
            "--help" | "-h" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown option: {other} (try --help)")),
        }
    }
    Ok(args)
}

fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The admin is a throwaway key for the demo. It is deliberately not your Solana CLI keypair.
fn load_or_create_admin(path: &Path) -> Result<Keypair, String> {
    if path.exists() {
        return read_keypair_file(path).map_err(|e| format!("{}: {e}", path.display()));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let admin = Keypair::new();
    write_keypair_file(&admin, path).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("Created demo admin key {} ({})", path.display(), admin.pubkey());
    Ok(admin)
}

fn send(client: &RpcClient, payer: &Keypair, instructions: &[Instruction]) -> Result<(), String> {
    let blockhash = client.get_latest_blockhash().map_err(|e| e.to_string())?;
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );
    client.send_and_confirm_transaction(&tx).map(|_| ()).map_err(|e| e.to_string())
}

/// Fetches accounts in chunks of 100 (the RPC limit).
fn fetch_accounts(
    client: &RpcClient,
    keys: &[Pubkey],
) -> Result<Vec<Option<solana_account::Account>>, String> {
    let mut out = Vec::with_capacity(keys.len());
    for chunk in keys.chunks(100) {
        out.extend(client.get_multiple_accounts(chunk).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Every prediction of the season that is on-chain, including ones people submitted through the
/// dashboard (they are not in the CSV). Layout: 8-byte tag, `user` (32), then `season` at byte 40.
fn season_predictions(
    client: &RpcClient,
    program_id: &Pubkey,
    season_key: &Pubkey,
) -> Result<Vec<(Pubkey, Prediction)>, String> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(0, Prediction::DISCRIMINATOR.to_vec())),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(40, season_key.to_bytes().to_vec())),
        ]),
        ..Default::default()
    };
    let accounts = client
        .get_program_ui_accounts_with_config(program_id, config)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(accounts.len());
    for (key, account) in accounts {
        let data = account.data.decode().ok_or("could not decode a prediction account")?;
        let prediction =
            Prediction::try_deserialize(&mut data.as_slice()).map_err(|e| e.to_string())?;
        out.push((key, prediction));
    }
    out.sort_by(|a, b| a.1.username.cmp(&b.1.username).then(a.0.cmp(&b.0)));
    Ok(out)
}

fn ensure_funded(client: &RpcClient, key: &Pubkey, minimum: u64) -> Result<(), String> {
    if client.get_balance(key).map_err(|e| e.to_string())? >= minimum {
        return Ok(());
    }
    println!("Airdropping {} SOL to the admin", 2 * minimum / LAMPORTS_PER_SOL);
    client.request_airdrop(key, 2 * minimum).map_err(|e| format!("airdrop failed: {e}"))?;
    for _ in 0..60 {
        if client.get_balance(key).map_err(|e| e.to_string())? >= minimum {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err("airdrop did not arrive in 30 seconds".into())
}

struct Predictor {
    row: PredictionRow,
    wallet: Keypair,
    account: Pubkey,
}

fn read_season(client: &RpcClient, key: &Pubkey) -> Result<Option<Season>, String> {
    let accounts = fetch_accounts(client, &[*key])?;
    match &accounts[0] {
        Some(a) => Ok(Some(Season::try_deserialize(&mut a.data.as_slice()).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    if !data::is_local_url(&args.rpc) && !args.allow_remote {
        return Err(format!(
            "{} is not a local RPC. This tool writes real people's names on-chain, so it only \
             talks to localhost unless you pass --allow-remote.",
            args.rpc
        ));
    }

    let rows = data::parse_predictions(&read_file(&args.predictions)?)?;
    let days = data::parse_standings(&read_file(&args.standings)?)?;
    {
        let mut names: Vec<&str> = rows.iter().map(|r| r.username.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        if names.len() != rows.len() {
            return Err("duplicate usernames in the predictions file".into());
        }
    }
    println!("Read {} predictions and {} game days", rows.len(), days.len());

    let client = RpcClient::new_with_commitment(args.rpc.clone(), CommitmentConfig::confirmed());
    client.get_version().map_err(|e| {
        format!("cannot reach {}: {e}\nIs solana-test-validator running? (see --help)", args.rpc)
    })?;

    let program_id = prediction_league::id();
    if fetch_accounts(&client, &[program_id])?[0].is_none() {
        return Err(format!(
            "program {program_id} is not deployed on {}. Start the validator with --bpf-program \
             (see --help).",
            args.rpc
        ));
    }

    let admin = load_or_create_admin(&args.admin)?;
    ensure_funded(&client, &admin.pubkey(), 5 * LAMPORTS_PER_SOL)?;

    let (season_key, _) =
        Pubkey::find_program_address(&[b"season", &args.season_id.to_le_bytes()], &program_id);

    // 1. Season
    let season = match read_season(&client, &season_key)? {
        Some(season) => {
            println!("Season {} exists (last day on-chain: {})", args.season_id, season.last_day);
            season
        }
        None => {
            let deadline = unix_now() + args.deadline_minutes * 60;
            let ix = Instruction::new_with_bytes(
                program_id,
                &prediction_league::instruction::CreateSeason {
                    season_id: args.season_id,
                    deadline,
                }
                .data(),
                prediction_league::accounts::CreateSeason {
                    admin: admin.pubkey(),
                    season: season_key,
                    system_program: system_program::ID,
                }
                .to_account_metas(None),
            );
            send(&client, &admin, &[ix])?;
            println!(
                "Created season {}, predictions open for {} minutes",
                args.season_id, args.deadline_minutes
            );
            read_season(&client, &season_key)?.ok_or("season missing after creation")?
        }
    };
    if season.admin != admin.pubkey() {
        return Err(format!(
            "season {} belongs to admin {}, not {}. Use the matching --admin-keypair or reset the validator.",
            args.season_id,
            season.admin,
            admin.pubkey()
        ));
    }

    // 2. Predictions
    let predictors: Vec<Predictor> = rows
        .into_iter()
        .map(|row| {
            let wallet = Keypair::new_from_array(data::wallet_seed(args.season_id, &row.username));
            let (account, _) = Pubkey::find_program_address(
                &[b"prediction", season_key.as_ref(), wallet.pubkey().as_ref()],
                &program_id,
            );
            Predictor { row, wallet, account }
        })
        .collect();
    seed_predictions(&client, &admin, program_id, season_key, &season, &predictors)?;

    // 3. Daily standings
    post_days(&client, &admin, program_id, season_key, &season, &days)?;

    // 4. Finalize (only when asked)
    let mut season = read_season(&client, &season_key)?.ok_or("season disappeared")?;
    if args.finalize {
        finalize(&client, &admin, program_id, season_key, &season)?;
        season = read_season(&client, &season_key)?.ok_or("season disappeared")?;
    }

    summary(&client, &program_id, &season_key, &season)
}

fn seed_predictions(
    client: &RpcClient,
    admin: &Keypair,
    program_id: Pubkey,
    season_key: Pubkey,
    season: &Season,
    predictors: &[Predictor],
) -> Result<(), String> {
    let keys: Vec<Pubkey> = predictors.iter().map(|p| p.account).collect();
    let existing = fetch_accounts(client, &keys)?;
    let missing: Vec<&Predictor> = predictors
        .iter()
        .zip(&existing)
        .filter(|(_, account)| account.is_none())
        .map(|(p, _)| p)
        .collect();

    if missing.is_empty() {
        println!("All {} predictions are already on-chain", predictors.len());
        return Ok(());
    }
    if unix_now() >= season.deadline {
        return Err(format!(
            "{} predictions are missing but the prediction deadline has passed, so they cannot be \
             added to this season. Reset the validator and seed again from scratch.",
            missing.len()
        ));
    }
    println!("Submitting {} predictions ({} already there)", missing.len(), predictors.len() - missing.len());

    // Fund the wallets that need it, several transfers per transaction.
    let wallets: Vec<Pubkey> = missing.iter().map(|p| p.wallet.pubkey()).collect();
    let balances = fetch_accounts(client, &wallets)?;
    let to_fund: Vec<Pubkey> = wallets
        .iter()
        .zip(&balances)
        .filter(|(_, acc)| acc.as_ref().map_or(0, |a| a.lamports) < PREDICTOR_FUNDING)
        .map(|(w, _)| *w)
        .collect();
    for chunk in to_fund.chunks(10) {
        let ixs: Vec<Instruction> = chunk
            .iter()
            .map(|w| system_instruction::transfer(&admin.pubkey(), w, PREDICTOR_FUNDING))
            .collect();
        send(client, admin, &ixs)?;
    }

    for (i, p) in missing.iter().enumerate() {
        let ix = Instruction::new_with_bytes(
            program_id,
            &prediction_league::instruction::SubmitPrediction {
                username: p.row.username.clone(),
                order: p.row.order,
            }
            .data(),
            prediction_league::accounts::SubmitPrediction {
                user: p.wallet.pubkey(),
                season: season_key,
                prediction: p.account,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        );
        send(client, &p.wallet, &[ix]).map_err(|e| format!("prediction for {}: {e}", p.row.username))?;
        if (i + 1) % 25 == 0 || i + 1 == missing.len() {
            println!("  {}/{} predictions submitted", i + 1, missing.len());
        }
    }
    Ok(())
}

fn daily_key(program_id: &Pubkey, season_key: &Pubkey, day_index: u16) -> Pubkey {
    Pubkey::find_program_address(
        &[b"daily", season_key.as_ref(), &day_index.to_le_bytes()],
        program_id,
    )
    .0
}

fn post_days(
    client: &RpcClient,
    admin: &Keypair,
    program_id: Pubkey,
    season_key: Pubkey,
    season: &Season,
    days: &[DayRow],
) -> Result<(), String> {
    let on_chain = season.last_day as usize;
    if days.len() < on_chain {
        return Err(format!(
            "the standings file has {} days but the chain already has {on_chain}. Is the file older than the chain?",
            days.len()
        ));
    }

    // Days already on-chain can never change, so make sure the file still agrees with them.
    // If NPB ever corrects an old game, this stops us from silently building on a different past.
    if on_chain > 0 {
        let keys: Vec<Pubkey> =
            (1..=on_chain as u16).map(|d| daily_key(&program_id, &season_key, d)).collect();
        for (day, account) in days.iter().zip(fetch_accounts(client, &keys)?) {
            let account = account.ok_or(format!("day {} missing on-chain", day.day_index))?;
            let stored = DailyResult::try_deserialize(&mut account.data.as_slice())
                .map_err(|e| e.to_string())?;
            if stored.date != day.date || stored.standings != day.standings {
                return Err(format!(
                    "day {} ({}) in the file differs from what is on-chain. On-chain days cannot be \
                     changed: reset the validator and seed again from scratch.",
                    day.day_index, day.date
                ));
            }
        }
    }

    let new_days = &days[on_chain..];
    if new_days.is_empty() {
        println!("All {} days are already on-chain", days.len());
        return Ok(());
    }
    println!("Posting {} new days ({} to {})", new_days.len(), new_days[0].date, new_days.last().unwrap().date);
    for (i, day) in new_days.iter().enumerate() {
        let ix = Instruction::new_with_bytes(
            program_id,
            &prediction_league::instruction::PostDailyResult {
                day_index: day.day_index,
                date: day.date,
                standings: day.standings,
            }
            .data(),
            prediction_league::accounts::PostDailyResult {
                admin: admin.pubkey(),
                season: season_key,
                daily_result: daily_key(&program_id, &season_key, day.day_index),
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        );
        send(client, admin, &[ix]).map_err(|e| format!("day {} ({}): {e}", day.day_index, day.date))?;
        if (i + 1) % 25 == 0 || i + 1 == new_days.len() {
            println!("  {}/{} days posted", i + 1, new_days.len());
        }
    }
    Ok(())
}

fn finalize(
    client: &RpcClient,
    admin: &Keypair,
    program_id: Pubkey,
    season_key: Pubkey,
    season: &Season,
) -> Result<(), String> {
    if !season.results_posted {
        let ix = Instruction::new_with_bytes(
            program_id,
            &prediction_league::instruction::FinalizeSeason {}.data(),
            prediction_league::accounts::FinalizeSeason { admin: admin.pubkey(), season: season_key }
                .to_account_metas(None),
        );
        send(client, admin, &[ix])?;
        println!("Season finalized with day {} as the final standings", season.last_day);
    }

    // Score everything on-chain for this season, not only the CSV pundits.
    let mut scored = 0;
    for (key, prediction) in season_predictions(client, &program_id, &season_key)? {
        if prediction.scored {
            continue;
        }
        let ix = Instruction::new_with_bytes(
            program_id,
            &prediction_league::instruction::ScorePrediction {}.data(),
            prediction_league::accounts::ScorePrediction {
                caller: admin.pubkey(),
                season: season_key,
                prediction: key,
            }
            .to_account_metas(None),
        );
        send(client, admin, &[ix]).map_err(|e| format!("scoring {}: {e}", prediction.username))?;
        scored += 1;
    }
    println!("Wrote the final score for {scored} predictions");
    Ok(())
}

/// Reads everything back from the chain and prints what a dashboard would show.
fn summary(
    client: &RpcClient,
    program_id: &Pubkey,
    season_key: &Pubkey,
    season: &Season,
) -> Result<(), String> {
    let mut scores: Vec<(String, u8, bool)> = Vec::new();
    for (_, p) in season_predictions(client, program_id, season_key)? {
        // On-chain `score` only exists after finalizing; otherwise show the score right now.
        let score = if p.scored { p.score } else { score_for(&p.order, &season.results) };
        scores.push((p.username, score, p.scored));
    }
    scores.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    println!();
    println!(
        "Season on-chain: {} days posted, season {}",
        season.last_day,
        if season.results_posted { "FINALIZED" } else { "still open (scores below are provisional)" }
    );
    let standings: Vec<&str> = season.results.iter().map(|&t| TEAM_NAMES[t as usize]).collect();
    println!("Latest standings: {}", standings.join(" > "));
    println!();
    println!("Top 5 of {} users:", scores.len());
    for (name, score, _) in scores.iter().take(5) {
        println!("  {score:>3}  {name}");
    }
    let mut histogram = std::collections::BTreeMap::new();
    for (_, score, _) in &scores {
        *histogram.entry(*score).or_insert(0) += 1;
    }
    let spread: Vec<String> = histogram.iter().rev().map(|(s, n)| format!("{s}:{n}")).collect();
    println!("Score spread (score:users): {}", spread.join("  "));
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
