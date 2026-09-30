use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Season {
    pub admin: Pubkey,
    pub season_id: u64,
    pub deadline: i64,
    pub results: [u8; 6],
    pub results_posted: bool,
    /// Last game day posted (0 = none yet).
    pub last_day: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct DailyResult {
    pub season: Pubkey,
    pub day_index: u16,
    /// YYYYMMDD, e.g. 20260328
    pub date: u32,
    /// standings[rank] = team index
    pub standings: [u8; 6],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Prediction {
    pub user: Pubkey,
    pub season: Pubkey,
    #[max_len(16)]
    pub username: String,
    pub order: [u8; 6],
    pub score: u8,
    pub scored: bool,
    pub bump: u8,
}
