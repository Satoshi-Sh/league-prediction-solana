import { Connection, Keypair } from '@solana/web3.js'
import { describe, expect, it } from 'vitest'
import { buildBoards } from './ranking.ts'
import { fetchLeague } from './chain.ts'
import { scoreFor } from './scoring.ts'
import { describeError, fetchMyPrediction, submitPrediction } from './submit.ts'

/**
 * Runs against a real cluster that the seeder has filled. Skipped unless INTEGRATION_RPC is set:
 *   INTEGRATION_RPC=http://127.0.0.1:8899 npm test
 * Optional: INTEGRATION_SEASON (default 2026).
 * The submit test adds a prediction to the chain on every run, so it also needs INTEGRATION_SUBMIT=1.
 */
const rpc = process.env.INTEGRATION_RPC
const seasonId = Number(process.env.INTEGRATION_SEASON ?? 2026)

describe.skipIf(!rpc)('against a seeded cluster', () => {
  it('reads the season, its days and its predictions', async () => {
    const league = await fetchLeague(rpc!, seasonId)
    expect(league).not.toBeNull()
    const { season, days, predictions } = league!

    expect(season.seasonId).toBe(seasonId)
    expect(days.length).toBe(season.lastDay)
    expect(days.map((d) => d.dayIndex)).toEqual(days.map((_, i) => i + 1))
    expect(predictions.length).toBeGreaterThan(0)
    for (const p of predictions) {
      expect([...p.order].sort()).toEqual([0, 1, 2, 3, 4, 5])
      expect(p.username.length).toBeGreaterThan(0)
    }
  })

  it('gives every user a board row on every day', async () => {
    const league = (await fetchLeague(rpc!, seasonId))!
    const boards = buildBoards(league.predictions, league.days)
    expect(boards).toHaveLength(league.days.length)
    for (const board of boards) expect(board.rows).toHaveLength(league.predictions.length)
  })

  it('matches the scores the program wrote, once the season is finalized', async ({ skip }) => {
    const league = (await fetchLeague(rpc!, seasonId))!
    if (!league.season.resultsPosted) skip('season is not finalized yet')

    const last = league.days[league.days.length - 1]
    // The final results are the last day's standings.
    expect(league.season.results).toEqual(last.standings)
    const mismatches = league.predictions.filter(
      (p) => !p.scored || p.score !== scoreFor(p.order, last.standings),
    )
    expect(mismatches.map((p) => p.username)).toEqual([])
  })
})

describe.skipIf(!rpc || !process.env.INTEGRATION_SUBMIT)('submitting a prediction', () => {
  it('lands on-chain, shows up in the league, and cannot be sent twice', async () => {
    const connection = new Connection(rpc!, 'confirmed')
    const wallet = Keypair.generate() // empty: submitPrediction must fund it from the faucet
    const order = [3, 2, 5, 0, 1, 4]
    const before = (await fetchLeague(rpc!, seasonId))!

    expect(await fetchMyPrediction(connection, seasonId, wallet.publicKey)).toBeNull()
    await submitPrediction(connection, wallet, seasonId, 'Integration Test', order)

    const mine = await fetchMyPrediction(connection, seasonId, wallet.publicKey)
    expect(mine).toMatchObject({
      user: wallet.publicKey.toBase58(),
      username: 'Integration Test',
      order,
      scored: false,
    })

    const after = (await fetchLeague(rpc!, seasonId))!
    expect(after.predictions.length).toBe(before.predictions.length + 1)
    const boards = buildBoards(after.predictions, after.days)
    const row = boards[boards.length - 1].rows.find((r) => r.username === 'Integration Test')
    expect(row?.score).toBe(scoreFor(order, after.days[after.days.length - 1].standings))

    // Same wallet again: the prediction account already exists.
    let message = ''
    await submitPrediction(connection, wallet, seasonId, 'Second try', order).catch((e: unknown) => {
      message = describeError(e)
    })
    expect(message).toMatch(/already submitted/)
  })

  it('refuses a blank name or a repeated team before sending anything', async () => {
    const connection = new Connection(rpc!, 'confirmed')
    const wallet = Keypair.generate()
    await expect(submitPrediction(connection, wallet, seasonId, '   ', [0, 1, 2, 3, 4, 5])).rejects.toThrow(/Enter a name/)
    await expect(submitPrediction(connection, wallet, seasonId, 'Ok', [0, 0, 2, 3, 4, 5])).rejects.toThrow(/exactly once/)
  })
})
