
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

    pub fn submit_prediction(ctx: Context<SubmitPrediction>, order: [u8; 6]) -> Result<()> {
        let season = &ctx.accounts.season;

        let now = Clock::get()?.unix_timestamp;
        require!(now < season.deadline, LeagueError::DeadlinePassed);
        require!(is_valid_order(&order), LeagueError::InvalidOrder);

        let prediction = &mut ctx.accounts.prediction;
        prediction.user = ctx.accounts.user.key();
        prediction.season = season.key();
        prediction.order = order;
        prediction.score = 0;
        prediction.scored = false;
        prediction.bump = ctx.bumps.prediction;
        Ok(())
    }

}



fn is_valid_order(order: &[u8; 6]) -> bool {
    let mut seen = [false; 6];
    for &team in order {
        if team >= 6 || seen[team as usize] {
            return false;
        }
        seen[team as usize] = true;
    }
    true
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

#[derive(Accounts)]
pub struct SubmitPrediction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub season: Account<'info, Season>,

    #[account(
        init,
        payer = user,
        space = 8 + Prediction::INIT_SPACE,
        seeds = [b"prediction", season.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub prediction: Account<'info, Prediction>,

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

#[account]
#[derive(InitSpace)]
pub struct Prediction {
    pub user: Pubkey,
    pub season: Pubkey,
    pub order: [u8; 6],
    pub score: u8,
    pub scored: bool,
    pub bump: u8,
}


#[error_code]
pub enum LeagueError {
    #[msg("The prediction deadline has passed")]
    DeadlinePassed,
    #[msg("Order must contain each team exactly once")]
    InvalidOrder,
}