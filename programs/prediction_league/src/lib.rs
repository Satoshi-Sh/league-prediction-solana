use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod scoring;
pub mod state;
pub mod validation;

pub use constants::*;
pub use error::*;
pub use instructions::*;
pub use state::*;

declare_id!("9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP");

#[program]
pub mod prediction_league {
    use super::*;

    pub fn create_season(ctx: Context<CreateSeason>, season_id: u64, deadline: i64) -> Result<()> {
        handle_create_season(ctx, season_id, deadline)
    }

    /// Admin posts the standings for one game day. Days must be posted in order (1, 2, 3, ...).
    /// `standings[rank] = team index`, so standings[0] is the team in first place.
    pub fn post_daily_result(
        ctx: Context<PostDailyResult>,
        day_index: u16,
        date: u32,
        standings: [u8; 6],
    ) -> Result<()> {
        handle_post_daily_result(ctx, day_index, date, standings)
    }

    pub fn submit_prediction(
        ctx: Context<SubmitPrediction>,
        username: String,
        order: [u8; 6],
    ) -> Result<()> {
        handle_submit_prediction(ctx, username, order)
    }

    /// Admin marks the season as over. The standings in `Season.results` (the last posted day)
    /// become the final result, and no more days can be posted.
    pub fn finalize_season(ctx: Context<FinalizeSeason>) -> Result<()> {
        handle_finalize_season(ctx)
    }

    /// Writes the final score of one prediction. Anyone can call this once the season is final.
    pub fn score_prediction(ctx: Context<ScorePrediction>) -> Result<()> {
        handle_score_prediction(ctx)
    }
}
