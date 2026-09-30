use anchor_lang::prelude::*;

// Keep new variants at the end: the order defines the on-chain error codes (6000, 6001, ...),
// and clients and tests depend on them.
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
    #[msg("The season is already finalized")]
    SeasonFinalized,
    #[msg("Post at least one daily result before finalizing")]
    NoResultsPosted,
    #[msg("The season has not been finalized yet")]
    SeasonNotFinalized,
    #[msg("This prediction has already been scored")]
    AlreadyScored,
}
