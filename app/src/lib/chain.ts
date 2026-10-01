import { Connection, PublicKey } from '@solana/web3.js'
import idl from '../idl/prediction_league.json'
import {
  decodeDailyResult,
  decodePrediction,
  decodeSeason,
  type Season,
} from './accounts.ts'
import type { DayData, PredictionData } from './ranking.ts'

export const PROGRAM_ID = new PublicKey(idl.address)

/** Everything the dashboard needs, read from the chain in one go. */
export interface League {
  season: Season
  days: DayData[]
  predictions: PredictionData[]
  fetchedAt: Date
}

function discriminator(accountName: string): Uint8Array {
  const account = idl.accounts.find((a) => a.name === accountName)
  if (!account) throw new Error(`${accountName} is not in the IDL`)
  return Uint8Array.from(account.discriminator)
}

function toBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
}

function u64le(value: number): Uint8Array {
  const bytes = new Uint8Array(8)
  new DataView(bytes.buffer).setBigUint64(0, BigInt(value), true)
  return bytes
}

export function seasonAddress(seasonId: number): PublicKey {
  return PublicKey.findProgramAddressSync(
    [new TextEncoder().encode('season'), u64le(seasonId)],
    PROGRAM_ID,
  )[0]
}

/** Filter on the 8-byte account type tag at the start of every Anchor account. */
function typeFilter(accountName: string) {
  return {
    memcmp: { offset: 0, bytes: toBase64(discriminator(accountName)), encoding: 'base64' as const },
  }
}

/** Just the season account (for its deadline and status). Null if it does not exist. */
export async function fetchSeason(connection: Connection, seasonId: number): Promise<Season | null> {
  const account = await connection.getAccountInfo(seasonAddress(seasonId))
  return account ? decodeSeason(account.data) : null
}

/**
 * Reads the season, all its daily results and all its predictions.
 * Returns null when the season account does not exist on this cluster.
 */
export async function fetchLeague(rpcUrl: string, seasonId: number): Promise<League | null> {
  const connection = new Connection(rpcUrl, 'confirmed')
  const seasonKey = seasonAddress(seasonId)

  const seasonAccount = await connection.getAccountInfo(seasonKey)
  if (!seasonAccount) return null
  const season = decodeSeason(seasonAccount.data)

  // DailyResult.season is at byte 8; Prediction.season is at byte 40 (after `user`).
  const [dailyAccounts, predictionAccounts] = await Promise.all([
    connection.getProgramAccounts(PROGRAM_ID, {
      filters: [typeFilter('DailyResult'), { memcmp: { offset: 8, bytes: seasonKey.toBase58() } }],
    }),
    connection.getProgramAccounts(PROGRAM_ID, {
      filters: [typeFilter('Prediction'), { memcmp: { offset: 40, bytes: seasonKey.toBase58() } }],
    }),
  ])

  const days: DayData[] = dailyAccounts
    .map(({ account }) => decodeDailyResult(account.data))
    .sort((a, b) => a.dayIndex - b.dayIndex)
    .map(({ dayIndex, date, standings }) => ({ dayIndex, date, standings }))

  const predictions: PredictionData[] = predictionAccounts.map(({ pubkey, account }) => {
    const p = decodePrediction(account.data)
    return {
      address: pubkey.toBase58(),
      user: p.user,
      username: p.username,
      order: p.order,
      score: p.score,
      scored: p.scored,
    }
  })

  return { season, days, predictions, fetchedAt: new Date() }
}
