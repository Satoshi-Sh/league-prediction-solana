//! Golden file shared with the dashboard: app/src/lib/scoring.test.ts reads the same JSON and
//! checks its TypeScript `scoreFor` against it.
//!
//! If you change the scoring rule, regenerate the file and commit it:
//!   UPDATE_FIXTURES=1 cargo test --test test_score_fixtures
//! Then `npm test` in app/ shows whether the TypeScript copy still agrees.

use prediction_league::scoring::score_for;
use std::{fs, path::PathBuf};

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/score_fixtures.json")
}

/// Small deterministic generator, so the fixture never changes without a reason.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn shuffled(rng: &mut Lcg) -> [u8; 6] {
    let mut order = [0u8, 1, 2, 3, 4, 5];
    for i in (1..6).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

fn generate() -> String {
    let identity = [0u8, 1, 2, 3, 4, 5];
    let mut cases: Vec<([u8; 6], [u8; 6])> = vec![
        (identity, identity),                    // perfect: 100
        ([5, 4, 3, 2, 1, 0], identity),          // fully reversed: 0
        ([1, 0, 2, 3, 4, 5], identity),          // one neighbour swap: 89
        ([1, 2, 3, 4, 5, 0], identity),          // 44
    ];
    let mut rng = Lcg(2026);
    for _ in 0..300 {
        cases.push((shuffled(&mut rng), shuffled(&mut rng)));
    }

    let lines: Vec<String> = cases
        .iter()
        .map(|(order, standings)| {
            // `{:?}` prints arrays as "[0, 1, 2]"; drop the spaces to keep one compact line per case.
            format!(
                "{{\"order\":{:?},\"standings\":{:?},\"score\":{}}}",
                order,
                standings,
                score_for(order, standings)
            )
            .replace(' ', "")
        })
        .collect();
    format!("[\n{}\n]\n", lines.join(",\n"))
}

#[test]
fn score_fixtures_match_the_rust_implementation() {
    let expected = generate();
    let path = fixtures_path();

    if std::env::var("UPDATE_FIXTURES").is_ok() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &expected).unwrap();
    }

    let on_disk = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing {}; run with UPDATE_FIXTURES=1", path.display()));
    assert_eq!(
        on_disk, expected,
        "scoring output changed. If that is intended, run: UPDATE_FIXTURES=1 cargo test --test test_score_fixtures"
    );
}
