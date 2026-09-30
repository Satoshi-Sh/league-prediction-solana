
use anchor_lang::prelude::*;

declare_id!("9kLrKPQLcRbeiEAPR9ttovQ2yXQ6rUYZdD63JQzqvLTP");

/// Max username length in bytes (must match `#[max_len]` on `Prediction::username`).
pub const MAX_USERNAME_LEN: usize = 16;

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
        season.last_day = 0;
        season.bump = ctx.bumps.season;
        Ok(())
    }

    /// Admin posts the standings for one game day. Days must be posted in order (1, 2, 3, ...).
    /// `standings[rank] = team index`, so standings[0] is the team in first place.
    pub fn post_daily_result(
        ctx: Context<PostDailyResult>,
        day_index: u16,
        date: u32,
        standings: [u8; 6],
    ) -> Result<()> {
        let season = &mut ctx.accounts.season;

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

    pub fn submit_prediction(
        ctx: Context<SubmitPrediction>,
        username: String,
        order: [u8; 6],
    ) -> Result<()> {
        let season = &ctx.accounts.season;

        let now = Clock::get()?.unix_timestamp;
        require!(now < season.deadline, LeagueError::DeadlinePassed);
        require!(is_valid_order(&order), LeagueError::InvalidOrder);
        require!(is_valid_username(&username), LeagueError::InvalidUsername);

        let prediction = &mut ctx.accounts.prediction;
        prediction.user = ctx.accounts.user.key();
        prediction.username = username;
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

/// 1 to MAX_USERNAME_LEN bytes, and not just whitespace. Duplicates are allowed.
fn is_valid_username(username: &str) -> bool {
    !username.trim().is_empty() && username.len() <= MAX_USERNAME_LEN
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


#[error_code]
pub enum LeagueError {
    #[msg("The prediction deadline has passed")]
    DeadlinePassed,
    #[msg("Order must contain each team exactly once")]
    InvalidOrder,
    #[msg("Only the season admin can do this")]
    Unauthorized,
    #[msg("Daily results must be posted in order, one day after the last")]
    DayOutOfOrder,
    #[msg("Username must be 1 to 16 bytes and not blank")]
    InvalidUsername,
}