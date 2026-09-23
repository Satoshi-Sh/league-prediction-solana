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

#[test]
fn test_create_season() {
    let program_id = prediction_league::id();
    let admin = Keypair::new();
    let season_id: u64 = 2026;
    let deadline: i64 = 1_775_000_000; // any Unix timestamp

    let season = Pubkey::find_program_address(
        &[b"season", &season_id.to_le_bytes()],
        &program_id,
    )
    .0;

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/prediction_league.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &prediction_league::instruction::CreateSeason { season_id, deadline }.data(),
        prediction_league::accounts::CreateSeason {
            admin: admin.pubkey(),
            season,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&admin.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&admin]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "{:?}", res.err());

    let account = svm.get_account(&season).unwrap();
    let mut data: &[u8] = &account.data;
    let stored = prediction_league::Season::try_deserialize(&mut data).unwrap();
    assert_eq!(stored.admin, admin.pubkey());
    assert_eq!(stored.season_id, season_id);
    assert_eq!(stored.deadline, deadline);
    assert!(!stored.results_posted);
}