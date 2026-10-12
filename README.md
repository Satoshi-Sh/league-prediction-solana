# Prediction League

Predict the final order of the six 2026 NPB Central League teams, then watch how each
prediction's score changes **every game day** of the season. The season, the daily
standings and the predictions live on Solana (devnet), and a React dashboard shows the
leaderboard.

Built for Solana School demo day by a lifelong Yomiuri Giants fan.

## How it works

```
NPB results ──► tools/npb_standings ──► data/2026_cl_standings.csv ─┐
ohtashp.com ──► tools/predictions   ──► data/2026_cl_predictions.csv ┤
                                                                     ▼
                                   tools/seeder ──► Anchor program (Solana) ──► app/ dashboard
```

- **Program** (`programs/prediction_league`): Anchor, with five instructions: `create_season`,
  `submit_prediction`, `post_daily_result`, `finalize_season`, `score_prediction`.
- **Score**: `round(100 * (18 - distance) / 18)`, where `distance` is how many places each team
  is off, added up. 100 means a perfect prediction.
- **Accounts** (PDAs): `Season`, one `DailyResult` per game day, one `Prediction` per wallet.
- **Dashboard** (`app/`): Vite, React and TypeScript. It reads the accounts straight from the
  RPC and computes the same score in TypeScript. A test checks Rust and TypeScript agree on
  the same 304 cases.
- **Teams** are numbered 0 Yomiuri, 1 Hanshin, 2 DeNA, 3 Hiroshima, 4 Chunichi, 5 Yakult.

## Repository layout

| Path | What it is |
| --- | --- |
| `programs/prediction_league` | The Anchor program and its tests |
| `tools/npb_standings` | Scrapes official NPB results into daily standings (Rust) |
| `tools/predictions` | Turns the pundit forecast page into a CSV (Python) |
| `tools/seeder` | Creates the season and posts predictions and daily results (Rust) |
| `app` | The dashboard and the submit page |
| `data` | Standings and prediction CSVs |

## Run the dashboard

You need Node.js. The dashboard reads from a local validator by default
(`http://127.0.0.1:8899`). To read from devnet instead, create `app/.env.local`:

```
VITE_RPC_URL=https://api.devnet.solana.com
```

Then:

```bash
cd app
npm install
npm run dev
```

Open the address Vite prints. The **Submit prediction** tab creates a throwaway wallet in the
browser and can ask the devnet faucet for test SOL.

Other commands: `npm test`, `npm run lint`, `npm run build`.

## Run it locally

You need Rust, the Solana CLI (with `cargo build-sbf`) and Anchor.

```bash
# 1. Build the program. The stock solana-test-validator needs the v1 build.
cd programs/prediction_league
cargo build-sbf --arch v1 --sbf-out-dir ../../target/deploy-v1
cd ../..

# 2. Start a local validator with the program loaded
solana-test-validator --reset --ledger /tmp/pl-ledger \
  --bpf-program 9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP target/deploy-v1/prediction_league.so

# 3. In another terminal: load the 2026 season (safe to re-run)
cargo run --manifest-path tools/seeder/Cargo.toml

# 4. Start the dashboard
cd app && npm run dev
```

When the season is over, run the seeder once more with `--finalize` to store the final scores.
See `cargo run --manifest-path tools/seeder/Cargo.toml -- --help` for all options.

## Run it on devnet

The program is deployed on devnet at
[`9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP`](https://explorer.solana.com/address/9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP?cluster=devnet).
To seed your own copy, deploy the v1 build with a funded wallet, then:

```bash
cargo run --manifest-path tools/seeder/Cargo.toml -- \
  --rpc https://api.devnet.solana.com --allow-remote \
  --admin-keypair ~/.config/solana/id.json \
  --predictions data/devnet_predictions.csv \
  --deadline-minutes 525600
```

The admin wallet pays for everything (a full season is a few SOL; the public devnet RPC is
slow, so expect several minutes). `data/devnet_predictions.csv` holds 10 pundit predictions;
`data/2026_cl_predictions.csv` holds all of them.

## Test

```bash
anchor build                 # the tests load target/deploy/prediction_league.so
cargo test                   # program, scoring and fixture tests
cargo test --manifest-path tools/npb_standings/Cargo.toml
cargo test --manifest-path tools/seeder/Cargo.toml
cd app && npm test
```

## Refresh the data

```bash
# Rebuild daily standings from the official NPB site (cached in data/cache)
cargo run --manifest-path tools/npb_standings/Cargo.toml

# Rebuild the pundit predictions from a saved copy of the forecast page
python3 tools/predictions/build_predictions.py path/to/baseball_yosou.md
```

The standings count only the 143 regular-season games per team; Climax Series games are ignored.
Teams with the same win percentage keep the previous day's order.

## Credits

Results: [NPB official site](https://npb.jp). Pundit forecasts: [ohtashp.com](https://www.ohtashp.com/topics/baseball_yosou/).
