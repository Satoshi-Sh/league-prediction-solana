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
    order: [u8; 6],
) -> (Instruction, Pubkey) {
    let prediction = Pubkey::find_program_address(
        &[b"prediction", season.as_ref(), user.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let ix = Instruction::new_with_bytes(
        program_id,
        &prediction_league::instruction::SubmitPrediction { order }.data(),
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
    let (ix, prediction) = submit_prediction_ix(program_id, season, &user, order);
    send(&mut svm, ix, &user).unwrap();

    let account = svm.get_account(&prediction).unwrap();
    let mut data: &[u8] = &account.data;
    let stored = prediction_league::Prediction::try_deserialize(&mut data).unwrap();
    assert_eq!(stored.user, user.pubkey());
    assert_eq!(stored.season, season);
    assert_eq!(stored.order, order);
    assert!(!stored.scored);
}

#[test]
fn test_duplicate_team_rejected() {
    let (mut svm, program_id, _admin, season) = setup();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 1_000_000_000).unwrap();

    // Yomiuri (0) appears twice
    let (ix, _) = submit_prediction_ix(program_id, season, &user, [0, 0, 2, 3, 4, 5]);
    let err = send(&mut svm, ix, &user).unwrap_err();

    // 6001 = LeagueError::InvalidOrder
    assert!(err.contains("Custom(6001)"), "unexpected error: {}", err);
}