use anchor_lang::prelude::*;

use crate::{
    error::LeagueError,
    scoring::score_for,
    state::{Prediction, Season},
};

#[derive(Accounts)]
pub struct ScorePrediction<'info> {
    pub caller: Signer<'info>,

    pub season: Account<'info, Season>,

    #[account(mut, has_one = season)]
    pub prediction: Account<'info, Prediction>,
}

/// Writes the final score of one prediction. Anyone can call this once the season is final.
pub fn handle_score_prediction(ctx: Context<ScorePrediction>) -> Result<()> {
    let season = &ctx.accounts.season;
    require!(season.results_posted, LeagueError::SeasonNotFinalized);

    let prediction = &mut ctx.accounts.prediction;
    require!(!prediction.scored, LeagueError::AlreadyScored);

    prediction.score = score_for(&prediction.order, &season.results);
    prediction.scored = true;
    Ok(())
}
