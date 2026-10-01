/**
 * Scoring, mirroring programs/prediction_league/src/scoring.rs. Keep the two in sync: a shared
 * fixture file (programs/prediction_league/tests/fixtures/score_fixtures.json) is checked by
 * tests on both sides.
 *
 * `order` and `standings` are lists of team indices by rank: order[0] is the team predicted
 * to finish first.
 *
 * distance = sum over teams of |predicted rank - actual rank|
 * score    = round(100 * (MAX_DISTANCE - distance) / MAX_DISTANCE)
 */

export const TEAM_COUNT = 6
export const MAX_SCORE = 100
/** Largest possible distance for 6 teams (for example a fully reversed order). */
export const MAX_DISTANCE = (TEAM_COUNT * TEAM_COUNT) / 2

export function totalDistance(order: readonly number[], standings: readonly number[]): number {
  const actualRank = new Array<number>(TEAM_COUNT).fill(0)
  standings.forEach((team, rank) => {
    actualRank[team] = rank
  })
  let distance = 0
  order.forEach((team, predictedRank) => {
    distance += Math.abs(predictedRank - actualRank[team])
  })
  return distance
}

export function scoreFor(order: readonly number[], standings: readonly number[]): number {
  const distance = totalDistance(order, standings)
  // Same integer rounding as the Rust code: add half the divisor, then divide.
  return Math.floor((MAX_SCORE * (MAX_DISTANCE - distance) + MAX_DISTANCE / 2) / MAX_DISTANCE)
}
