use anchor_lang::prelude::*;

use crate::{error::LeagueError, state::Season};

#[derive(Accounts)]
pub struct FinalizeSeason<'info> {
    pub admin: Signer<'info>,

    #[account(mut, has_one = admin @ LeagueError::Unauthorized)]
    pub season: Account<'info, Season>,
}

/// Admin marks the season as over. The standings in `Season.results` (the last posted day)
/// become the final result, and no more days can be posted.
pub fn handle_finalize_season(ctx: Context<FinalizeSeason>) -> Result<()> {
    let season = &mut ctx.accounts.season;
    require!(!season.results_posted, LeagueError::SeasonFinalized);
    require!(season.last_day > 0, LeagueError::NoResultsPosted);

    season.results_posted = true;
    Ok(())
}
