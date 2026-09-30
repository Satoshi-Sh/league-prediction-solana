use anchor_lang::prelude::*;

use crate::{
    error::LeagueError,
    state::{DailyResult, Season},
    validation::is_valid_order,
};

#[derive(Accounts)]
#[instruction(day_index: u16)]
pub struct PostDailyResult<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut, has_one = admin @ LeagueError::Unauthorized)]
    pub season: Account<'info, Season>,

    #[account(
        init,
        payer = admin,
        space = 8 + DailyResult::INIT_SPACE,
        seeds = [b"daily", season.key().as_ref(), day_index.to_le_bytes().as_ref()],
        bump,
    )]
    pub daily_result: Account<'info, DailyResult>,

    pub system_program: Program<'info, System>,
}

/// Admin posts the standings for one game day. Days must be posted in order (1, 2, 3, ...).
/// `standings[rank] = team index`, so standings[0] is the team in first place.
pub fn handle_post_daily_result(
    ctx: Context<PostDailyResult>,
    day_index: u16,
    date: u32,
    standings: [u8; 6],
) -> Result<()> {
    let season = &mut ctx.accounts.season;

    require!(!season.results_posted, LeagueError::SeasonFinalized);
    require!(
        day_index == season.last_day + 1,
        LeagueError::DayOutOfOrder
    );
    require!(is_valid_order(&standings), LeagueError::InvalidOrder);

    let daily = &mut ctx.accounts.daily_result;
    daily.season = season.key();
    daily.day_index = day_index;
    daily.date = date;
    daily.standings = standings;
    daily.bump = ctx.bumps.daily_result;

    // Keep the latest standings readable directly on the season account.
    season.last_day = day_index;
    season.results = standings;
    Ok(())
}
