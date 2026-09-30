use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const SEASON_ID: u64 = 2026;
const DEADLINE: i64 = 1_775_000_000;

/// Sends one instruction signed by `signer`. Returns the error as text if it fails.
fn send(svm: &mut LiteSVM, ix: Instruction, signer: &Keypair) -> Result<(), String> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.err))
}

/// Loads the program, funds an admin, and creates a season.
fn setup() -> (LiteSVM, Pubkey, Keypair, Pubkey) {
    let program_id = prediction_league::id();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/prediction_league.so"
    ));
    svm.add_program(program_id, bytes).unwrap();

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

    let season = Pubkey::find_program_address(
        &[b"season", &SEASON_ID.to_le_bytes()],
        &program_id,
    )
    .0;

    let ix = Instruction::new_with_bytes(
        program_id,
        &prediction_league::instruction::CreateSeason {
            season_id: SEASON_ID,
            deadline: DEADLINE,
        }
        .data(),
        prediction_league::accounts::CreateSeason {
            admin: admin.pubkey(),
            season,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(&mut svm, ix, &admin).unwrap();

    (svm, program_id, admin, season)
}

/// Builds a submit_prediction instruction and returns it with the prediction's address.
fn submit_prediction_ix(
    program_id: Pubkey,
    season: Pubkey,
    user: &Keypair,
    username: &str,
    order: [u8; 6],
) -> (Instruction, Pubkey) {
    let prediction = Pubkey::find_program_address(
        &[b"prediction", season.as_ref(), user.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let ix = Instruction::new_with_bytes(
        program_id,
        &prediction_league::instruction::SubmitPrediction {
            username: username.to_string(),
            order,
        }
        .data(),
        prediction_league::accounts::SubmitPrediction {
            user: user.pubkey(),
            season,
            prediction,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    (ix, prediction)
}

#[test]
fn test_create_season() {
    let (svm, _program_id, admin, season) = setup();

    let account = svm.get_account(&season).unwrap();
    let mut data: &[u8] = &account.data;
    let stored = prediction_league::Season::try_deserialize(&mut data).unwrap();
    assert_eq!(stored.admin, admin.pubkey());
    assert_eq!(stored.season_id, SEASON_ID);
    assert_eq!(stored.deadline, DEADLINE);
    assert!(!stored.results_posted);
}

#[test]
fn test_submit_prediction() {
    let (mut svm, program_id, _admin, season) = setup();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // 0=Yomiuri, 1=Hanshin, 2=DeNA, 3=Hiroshima, 4=Chunichi, 5=Yakult
    let order = [0, 1, 2, 3, 4, 5];
    let (ix, prediction) = submit_prediction_ix(program_id, season, &user, "satoshi", order);
    send(&mut svm, ix, &user).unwrap();

    let account = svm.get_account(&prediction).unwrap();
    let mut data: &[u8] = &account.data;
    let stored = prediction_league::Prediction::try_deserialize(&mut data).unwrap();
    assert_eq!(stored.user, user.pubkey());
    assert_eq!(stored.season, season);
    assert_eq!(stored.username, "satoshi");
    assert_eq!(stored.order, order);
    assert!(!stored.scored);
}

#[test]
fn test_duplicate_team_rejected() {
    let (mut svm, program_id, _admin, season) = setup();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // Yomiuri (0) appears twice
    let (ix, _) = submit_prediction_ix(program_id, season, &user, "satoshi", [0, 0, 2, 3, 4, 5]);
    let err = send(&mut svm, ix, &user).unwrap_err();

    // 6001 = LeagueError::InvalidOrder
    assert!(err.contains("Custom(6001)"), "unexpected error: {}", err);
}

/// Builds a post_daily_result instruction and returns it with the daily account's address.
fn post_daily_ix(
    program_id: Pubkey,
    season: Pubkey,
    admin: &Keypair,
    day_index: u16,
    date: u32,
    standings: [u8; 6],
) -> (Instruction, Pubkey) {
    let daily_result = Pubkey::find_program_address(
        &[b"daily", season.as_ref(), &day_index.to_le_bytes()],
        &program_id,
    )
    .0;

    let ix = Instruction::new_with_bytes(
        program_id,
        &prediction_league::instruction::PostDailyResult {
            day_index,
            date,
            standings,
        }
        .data(),
        prediction_league::accounts::PostDailyResult {
            admin: admin.pubkey(),
            season,
            daily_result,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    (ix, daily_result)
}

#[test]
fn test_post_daily_results() {
    let (mut svm, program_id, admin, season) = setup();

    let day1 = [1, 0, 2, 3, 4, 5];
    let day2 = [0, 1, 2, 3, 5, 4];

    let (ix, daily1) = post_daily_ix(program_id, season, &admin, 1, 20260328, day1);
    send(&mut svm, ix, &admin).unwrap();
    let (ix, daily2) = post_daily_ix(program_id, season, &admin, 2, 20260329, day2);
    send(&mut svm, ix, &admin).unwrap();

    // Each day keeps its own standings.
    for (addr, day_index, date, standings) in [
        (daily1, 1u16, 20260328u32, day1),
        (daily2, 2, 20260329, day2),
    ] {
        let account = svm.get_account(&addr).unwrap();
        let mut data: &[u8] = &account.data;
        let stored = prediction_league::DailyResult::try_deserialize(&mut data).unwrap();
        assert_eq!(stored.season, season);
        assert_eq!(stored.day_index, day_index);
        assert_eq!(stored.date, date);
        assert_eq!(stored.standings, standings);
    }

    // Season tracks the latest day and standings.
    let account = svm.get_account(&season).unwrap();
    let mut data: &[u8] = &account.data;
    let stored = prediction_league::Season::try_deserialize(&mut data).unwrap();
    assert_eq!(stored.last_day, 2);
    assert_eq!(stored.results, day2);
}

#[test]
fn test_post_daily_result_non_admin_rejected() {
    let (mut svm, program_id, _admin, season) = setup();
    let intruder = Keypair::new();
    svm.airdrop(&intruder.pubkey(), 1_000_000_000).unwrap();

    let (ix, _) = post_daily_ix(program_id, season, &intruder, 1, 20260328, [0, 1, 2, 3, 4, 5]);
    let err = send(&mut svm, ix, &intruder).unwrap_err();

    // 6002 = LeagueError::Unauthorized
    assert!(err.contains("Custom(6002)"), "unexpected error: {}", err);
}

#[test]
fn test_post_daily_result_skipped_day_rejected() {
    let (mut svm, program_id, admin, season) = setup();

    // Day 1 was never posted, so day 2 must fail.
    let (ix, _) = post_daily_ix(program_id, season, &admin, 2, 20260329, [0, 1, 2, 3, 4, 5]);
    let err = send(&mut svm, ix, &admin).unwrap_err();

    // 6003 = LeagueError::DayOutOfOrder
    assert!(err.contains("Custom(6003)"), "unexpected error: {}", err);
}

#[test]
fn test_post_daily_result_invalid_standings_rejected() {
    let (mut svm, program_id, admin, season) = setup();

    let (ix, _) = post_daily_ix(program_id, season, &admin, 1, 20260328, [0, 0, 2, 3, 4, 5]);
    let err = send(&mut svm, ix, &admin).unwrap_err();

    // 6001 = LeagueError::InvalidOrder
    assert!(err.contains("Custom(6001)"), "unexpected error: {}", err);
}


#[test]
fn test_invalid_username_rejected() {
    // 6004 = LeagueError::InvalidUsername
    let too_long = "a".repeat(17);
    for bad in ["", "   ", too_long.as_str()] {
        let (mut svm, program_id, _admin, season) = setup();
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

        let (ix, _) = submit_prediction_ix(program_id, season, &user, bad, [0, 1, 2, 3, 4, 5]);
        let err = send(&mut svm, ix, &user).unwrap_err();
        assert!(
            err.contains("Custom(6004)"),
            "username {:?}: unexpected error: {}",
            bad,
            err
        );
    }
}

#[test]
fn test_username_max_length_and_duplicates_allowed() {
    let (mut svm, program_id, _admin, season) = setup();
    let exactly_16 = "a".repeat(16);

    // Two different wallets may use the same name, and 16 bytes is accepted.
    for _ in 0..2 {
        let user = Keypair::new();
        svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();
        let (ix, prediction) =
            submit_prediction_ix(program_id, season, &user, &exactly_16, [0, 1, 2, 3, 4, 5]);
        send(&mut svm, ix, &user).unwrap();

        let account = svm.get_account(&prediction).unwrap();
        let mut data: &[u8] = &account.data;
        let stored = prediction_league::Prediction::try_deserialize(&mut data).unwrap();
        assert_eq!(stored.username, exactly_16);
    }
}
