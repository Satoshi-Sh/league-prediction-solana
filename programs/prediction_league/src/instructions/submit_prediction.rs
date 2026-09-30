use anchor_lang::prelude::*;

use crate::{
    error::LeagueError,
    state::{Prediction, Season},
    validation::{is_valid_order, is_valid_username},
};

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

pub fn handle_submit_prediction(
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
