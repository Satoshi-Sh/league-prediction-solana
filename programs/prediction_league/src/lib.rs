
use anchor_lang::prelude::*;

declare_id!("9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP");

#[program]
pub mod prediction_league {
    use super::*;

    pub fn create_season(ctx: Context<CreateSeason>, season_id: u64, deadline: i64) -> Result<()> {
        let season = &mut ctx.accounts.season;
        season.admin = ctx.accounts.admin.key();
        season.season_id = season_id;
        season.deadline = deadline;
        season.results = [0; 6];
        season.results_posted = false;
        season.bump = ctx.bumps.season;
        Ok(())
    }

}

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

#[account]
#[derive(InitSpace)]
pub struct Season {
    pub admin: Pubkey,
    pub season_id: u64,
    pub deadline: i64,
    pub results: [u8; 6],
    pub results_posted: bool,
    pub bump: u8,
}