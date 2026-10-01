import { describe, expect, it } from 'vitest'
import {
  buildBoards,
  competitionRanks,
  formatDate,
  seriesFor,
  type DayData,
  type PredictionData,
} from './ranking.ts'

function prediction(name: string, order: number[]): PredictionData {
  return { address: `addr-${name}`, user: `user-${name}`, username: name, order, score: 0, scored: false }
}

describe('competitionRanks', () => {
  it('gives tied users the same rank and skips the following ranks', () => {
    expect(competitionRanks([89, 89, 78, 78, 78, 11])).toEqual([1, 1, 3, 3, 3, 6])
  })

  it('handles a single user and all-equal scores', () => {
    expect(competitionRanks([50])).toEqual([1])
    expect(competitionRanks([10, 10, 10])).toEqual([1, 1, 1])
  })
})

describe('buildBoards', () => {
  const identity = [0, 1, 2, 3, 4, 5]
  const swapped = [1, 0, 2, 3, 4, 5]
  const reversed = [5, 4, 3, 2, 1, 0]

  const predictions = [
    prediction('Ace', identity),
    prediction('Bee', swapped),
    prediction('Cat', reversed),
  ]

  // Day 1: identity standings, so Ace is perfect. Day 2: Bee's order becomes the real standings.
  const days: DayData[] = [
    { dayIndex: 1, date: 20260327, standings: identity },
    { dayIndex: 2, date: 20260328, standings: swapped },
  ]

  it('scores every user against each day, not against the final standings', () => {
    const [day1, day2] = buildBoards(predictions, days)
    const score = (board: typeof day1, name: string) => board.rows.find((r) => r.username === name)!.score
    expect(score(day1, 'Ace')).toBe(100)
    expect(score(day1, 'Bee')).toBe(89)
    expect(score(day2, 'Ace')).toBe(89)
    expect(score(day2, 'Bee')).toBe(100)
  })

  it('sorts by rank then username and uses shared ranks', () => {
    const [day1] = buildBoards(predictions, days)
    expect(day1.rows.map((r) => [r.username, r.rank])).toEqual([
      ['Ace', 1],
      ['Bee', 2],
      ['Cat', 3],
    ])
  })

  it('has no movement on day 1', () => {
    const [day1] = buildBoards(predictions, days)
    expect(day1.rows.every((r) => r.movement === null && r.scoreDelta === null)).toBe(true)
    expect(day1.teamMovement.every((m) => m === null)).toBe(true)
  })

  it('reports movement as previous rank minus current rank (positive = up)', () => {
    const [, day2] = buildBoards(predictions, days)
    const row = (name: string) => day2.rows.find((r) => r.username === name)!
    expect(row('Bee').rank).toBe(1)
    expect(row('Bee').movement).toBe(1) // 2nd -> 1st
    expect(row('Bee').scoreDelta).toBe(11)
    expect(row('Ace').rank).toBe(2)
    expect(row('Ace').movement).toBe(-1) // 1st -> 2nd
    expect(row('Ace').scoreDelta).toBe(-11)
    expect(row('Cat').movement).toBe(0)
  })

  it('reports team movement from the standings', () => {
    const [, day2] = buildBoards(predictions, days)
    // Team 1 went from 2nd (rank index 1) to 1st (index 0): up one place. Team 0 down one.
    expect(day2.teamMovement).toEqual([-1, 1, 0, 0, 0, 0])
  })

  it('compares shared ranks for movement when users tie', () => {
    const twins = [prediction('Twin1', identity), prediction('Twin2', identity), prediction('Low', reversed)]
    const boards = buildBoards(twins, [
      { dayIndex: 1, date: 20260327, standings: identity },
      { dayIndex: 2, date: 20260328, standings: identity },
    ])
    expect(boards[1].rows.map((r) => r.rank)).toEqual([1, 1, 3])
    expect(boards[1].rows.every((r) => r.movement === 0)).toBe(true)
  })

  it('returns no boards when there are no days, and empty rows when there are no predictions', () => {
    expect(buildBoards(predictions, [])).toEqual([])
    expect(buildBoards([], days)[0].rows).toEqual([])
  })
})

describe('seriesFor and formatDate', () => {
  it('builds one point per day for a user', () => {
    const boards = buildBoards(
      [prediction('Ace', [0, 1, 2, 3, 4, 5])],
      [
        { dayIndex: 1, date: 20260327, standings: [0, 1, 2, 3, 4, 5] },
        { dayIndex: 2, date: 20260328, standings: [1, 0, 2, 3, 4, 5] },
      ],
    )
    expect(seriesFor(boards, 'addr-Ace').map((p) => [p.dayIndex, p.score, p.rank])).toEqual([
      [1, 100, 1],
      [2, 89, 1],
    ])
    expect(seriesFor(boards, 'missing')).toEqual([])
  })

  it('formats dates', () => {
    expect(formatDate(20260327)).toBe('2026-03-27')
  })
})
