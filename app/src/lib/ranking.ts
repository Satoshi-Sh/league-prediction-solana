import { scoreFor } from './scoring.ts'

export interface PredictionData {
  /** Address of the Prediction account. */
  address: string
  /** Wallet that submitted it. */
  user: string
  username: string
  order: number[]
  /** Final score written on-chain. Only meaningful when `scored` is true. */
  score: number
  scored: boolean
}

export interface DayData {
  dayIndex: number
  /** YYYYMMDD */
  date: number
  /** standings[rank] = team index */
  standings: number[]
}

export interface BoardRow {
  address: string
  username: string
  order: number[]
  score: number
  /** Shared rank: users with the same score have the same rank (1, 2, 2, 4). */
  rank: number
  /** Positive = moved up since the previous day. null on the first day. */
  movement: number | null
  /** Score change since the previous day. null on the first day. */
  scoreDelta: number | null
}

export interface DayBoard {
  dayIndex: number
  date: number
  standings: number[]
  rows: BoardRow[]
  /** Per team index: places gained since the previous day (positive = up). null on day 1. */
  teamMovement: (number | null)[]
}

/**
 * Competition ranking: rank = 1 + number of users with a strictly higher score.
 * Scores 89, 89, 78 give ranks 1, 1, 3.
 */
export function competitionRanks(scores: readonly number[]): number[] {
  return scores.map((score) => 1 + scores.filter((other) => other > score).length)
}

function rankOfTeams(standings: readonly number[]): number[] {
  const ranks = new Array<number>(standings.length).fill(0)
  standings.forEach((team, rank) => {
    ranks[team] = rank
  })
  return ranks
}

/**
 * Builds the leaderboard for every game day. Days must be in order. Scores for each day are
 * computed from that day's standings, so the history can be replayed from on-chain accounts.
 */
export function buildBoards(predictions: readonly PredictionData[], days: readonly DayData[]): DayBoard[] {
  const boards: DayBoard[] = []
  let previousRank = new Map<string, number>()
  let previousScore = new Map<string, number>()
  let previousTeamRank: number[] | null = null

  for (const day of days) {
    const scores = predictions.map((p) => scoreFor(p.order, day.standings))
    const ranks = competitionRanks(scores)

    const rows: BoardRow[] = predictions.map((p, i) => ({
      address: p.address,
      username: p.username,
      order: p.order,
      score: scores[i],
      rank: ranks[i],
      movement: previousRank.has(p.address) ? previousRank.get(p.address)! - ranks[i] : null,
      scoreDelta: previousScore.has(p.address) ? scores[i] - previousScore.get(p.address)! : null,
    }))
    rows.sort((a, b) => a.rank - b.rank || a.username.localeCompare(b.username))

    const teamRank = rankOfTeams(day.standings)
    const teamMovement = teamRank.map((rank, team) =>
      previousTeamRank ? previousTeamRank[team] - rank : null,
    )

    boards.push({ dayIndex: day.dayIndex, date: day.date, standings: day.standings, rows, teamMovement })

    previousRank = new Map(rows.map((r) => [r.address, r.rank]))
    previousScore = new Map(rows.map((r) => [r.address, r.score]))
    previousTeamRank = teamRank
  }
  return boards
}

export interface SeriesPoint {
  dayIndex: number
  date: number
  score: number
  rank: number
}

/** One user's score and rank on every day, for the chart. */
export function seriesFor(boards: readonly DayBoard[], address: string): SeriesPoint[] {
  return boards.flatMap((board) => {
    const row = board.rows.find((r) => r.address === address)
    return row ? [{ dayIndex: board.dayIndex, date: board.date, score: row.score, rank: row.rank }] : []
  })
}

/** 20260327 -> "2026-03-27" */
export function formatDate(date: number): string {
  const s = String(date)
  return `${s.slice(0, 4)}-${s.slice(4, 6)}-${s.slice(6, 8)}`
}
