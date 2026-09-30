use anchor_lang::prelude::*;

use crate::state::Season;

#[derive(Accounts)]
#[instruction(season_id: u64)]
pub struct CreateSeason<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + Season::INIT_SPACE,
        seeds = [b"season", season_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub season: Account<'info, Season>,

    pub system_program: Program<'info, System>,
}

pub fn handle_create_season(
    ctx: Context<CreateSeason>,
    season_id: u64,
    deadline: i64,
) -> Result<()> {
    let season = &mut ctx.accounts.season;
    season.admin = ctx.accounts.admin.key();
    season.season_id = season_id;
    season.deadline = deadline;
    season.results = [0; 6];
    season.results_posted = false;
    season.last_day = 0;
    season.bump = ctx.bumps.season;
    Ok(())
}
