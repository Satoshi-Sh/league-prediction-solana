/**
 * Decoders for the three account types of the prediction_league program. The layouts are
 * Borsh (as written by Anchor): an 8-byte discriminator, then the fields in declaration order.
 * See src/idl/prediction_league.json and programs/prediction_league/src/state.rs.
 *
 * Decoding by hand keeps the app free of the Anchor client library, and every account here is
 * fixed-size apart from one string.
 */
import { PublicKey } from '@solana/web3.js'

const DISCRIMINATOR_LEN = 8

/** Reads little-endian fields one after another from a byte array. */
class Reader {
  private offset = DISCRIMINATOR_LEN
  private readonly view: DataView
  private readonly data: Uint8Array

  constructor(data: Uint8Array) {
    this.data = data
    this.view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  }

  pubkey(): string {
    const bytes = this.data.slice(this.offset, this.offset + 32)
    this.offset += 32
    return new PublicKey(bytes).toBase58()
  }
  u8(): number {
    return this.view.getUint8(this.offset++)
  }
  bool(): boolean {
    return this.u8() !== 0
  }
  u16(): number {
    const v = this.view.getUint16(this.offset, true)
    this.offset += 2
    return v
  }
  u32(): number {
    const v = this.view.getUint32(this.offset, true)
    this.offset += 4
    return v
  }
  u64(): number {
    const v = this.view.getBigUint64(this.offset, true)
    this.offset += 8
    return Number(v)
  }
  i64(): number {
    const v = this.view.getBigInt64(this.offset, true)
    this.offset += 8
    return Number(v)
  }
  bytes(n: number): number[] {
    const out = Array.from(this.data.slice(this.offset, this.offset + n))
    this.offset += n
    return out
  }
  string(): string {
    const length = this.u32()
    const text = new TextDecoder().decode(this.data.slice(this.offset, this.offset + length))
    this.offset += length
    return text
  }
}

export interface Season {
  admin: string
  seasonId: number
  /** Unix seconds: predictions close at this time. */
  deadline: number
  /** Latest posted standings (final once `resultsPosted`). */
  results: number[]
  /** True once the admin finalized the season. */
  resultsPosted: boolean
  lastDay: number
}

export interface DailyResultAccount {
  season: string
  dayIndex: number
  date: number
  standings: number[]
}

export interface PredictionAccount {
  user: string
  season: string
  username: string
  order: number[]
  score: number
  scored: boolean
}

export function decodeSeason(data: Uint8Array): Season {
  const r = new Reader(data)
  const admin = r.pubkey()
  const seasonId = r.u64()
  const deadline = r.i64()
  const results = r.bytes(6)
  const resultsPosted = r.bool()
  const lastDay = r.u16()
  return { admin, seasonId, deadline, results, resultsPosted, lastDay }
}

export function decodeDailyResult(data: Uint8Array): DailyResultAccount {
  const r = new Reader(data)
  const season = r.pubkey()
  const dayIndex = r.u16()
  const date = r.u32()
  const standings = r.bytes(6)
  return { season, dayIndex, date, standings }
}

export function decodePrediction(data: Uint8Array): PredictionAccount {
  const r = new Reader(data)
  const user = r.pubkey()
  const season = r.pubkey()
  const username = r.string()
  const order = r.bytes(6)
  const score = r.u8()
  const scored = r.bool()
  return { user, season, username, order, score, scored }
}
