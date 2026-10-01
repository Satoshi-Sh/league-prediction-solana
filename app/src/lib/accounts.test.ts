import { PublicKey } from '@solana/web3.js'
import { describe, expect, it } from 'vitest'
import { decodeDailyResult, decodePrediction, decodeSeason } from './accounts.ts'

/** Small Borsh writer, the inverse of what the decoders read. */
class Writer {
  private parts: number[] = []
  bytes(values: ArrayLike<number>) {
    this.parts.push(...Array.from(values))
    return this
  }
  u8(v: number) {
    return this.bytes([v])
  }
  u16(v: number) {
    const b = new Uint8Array(2)
    new DataView(b.buffer).setUint16(0, v, true)
    return this.bytes(b)
  }
  u32(v: number) {
    const b = new Uint8Array(4)
    new DataView(b.buffer).setUint32(0, v, true)
    return this.bytes(b)
  }
  u64(v: number) {
    const b = new Uint8Array(8)
    new DataView(b.buffer).setBigUint64(0, BigInt(v), true)
    return this.bytes(b)
  }
  i64(v: number) {
    const b = new Uint8Array(8)
    new DataView(b.buffer).setBigInt64(0, BigInt(v), true)
    return this.bytes(b)
  }
  string(s: string) {
    const encoded = new TextEncoder().encode(s)
    return this.u32(encoded.length).bytes(encoded)
  }
  done() {
    return Uint8Array.from(this.parts)
  }
}

const DISCRIMINATOR = [1, 2, 3, 4, 5, 6, 7, 8]
const adminKey = new PublicKey(new Uint8Array(32).fill(7))
const seasonKey = new PublicKey(new Uint8Array(32).fill(9))
const userKey = new PublicKey(new Uint8Array(32).fill(3))

describe('account decoders', () => {
  it('decodes Season', () => {
    const data = new Writer()
      .bytes(DISCRIMINATOR)
      .bytes(adminKey.toBytes())
      .u64(2026)
      .i64(1_775_000_000)
      .bytes([1, 0, 4, 3, 2, 5])
      .u8(1) // results_posted
      .u16(158)
      .u8(254) // bump
      .done()
    expect(decodeSeason(data)).toEqual({
      admin: adminKey.toBase58(),
      seasonId: 2026,
      deadline: 1_775_000_000,
      results: [1, 0, 4, 3, 2, 5],
      resultsPosted: true,
      lastDay: 158,
    })
  })

  it('decodes DailyResult', () => {
    const data = new Writer()
      .bytes(DISCRIMINATOR)
      .bytes(seasonKey.toBytes())
      .u16(42)
      .u32(20260512)
      .bytes([3, 1, 0, 2, 5, 4])
      .u8(255)
      .done()
    expect(decodeDailyResult(data)).toEqual({
      season: seasonKey.toBase58(),
      dayIndex: 42,
      date: 20260512,
      standings: [3, 1, 0, 2, 5, 4],
    })
  })

  it('decodes Prediction including a non-ASCII username and unused trailing space', () => {
    // Anchor allocates room for the longest username, so the account can be longer than its data.
    const data = new Writer()
      .bytes(DISCRIMINATOR)
      .bytes(userKey.toBytes())
      .bytes(seasonKey.toBytes())
      .string('Iwase Hitoki')
      .bytes([1, 4, 3, 0, 2, 5])
      .u8(89)
      .u8(1)
      .u8(253)
      .bytes(new Uint8Array(4))
      .done()
    expect(decodePrediction(data)).toEqual({
      user: userKey.toBase58(),
      season: seasonKey.toBase58(),
      username: 'Iwase Hitoki',
      order: [1, 4, 3, 0, 2, 5],
      score: 89,
      scored: true,
    })

    const unicode = new Writer()
      .bytes(DISCRIMINATOR)
      .bytes(userKey.toBytes())
      .bytes(seasonKey.toBytes())
      .string('岩瀬仁紀')
      .bytes([0, 1, 2, 3, 4, 5])
      .u8(0)
      .u8(0)
      .u8(1)
      .done()
    expect(decodePrediction(unicode).username).toBe('岩瀬仁紀')
  })

  it('reads from a view into a larger buffer (as RPC clients return)', () => {
    const inner = new Writer()
      .bytes(DISCRIMINATOR)
      .bytes(seasonKey.toBytes())
      .u16(1)
      .u32(20260327)
      .bytes([0, 1, 2, 3, 4, 5])
      .u8(1)
      .done()
    const padded = new Uint8Array(inner.length + 20)
    padded.set(inner, 10)
    const view = padded.subarray(10, 10 + inner.length)
    expect(decodeDailyResult(view).date).toBe(20260327)
  })
})
