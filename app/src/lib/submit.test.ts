import { Keypair, SystemProgram } from '@solana/web3.js'
import { describe, expect, it } from 'vitest'
import idl from '../idl/prediction_league.json'
import { PROGRAM_ID, seasonAddress } from './chain.ts'
import {
  buildSubmitInstruction,
  describeError,
  encodeSubmitData,
  isValidOrder,
  MAX_USERNAME_BYTES,
  moveTeam,
  predictionAddress,
  usernameBytes,
  validateUsername,
} from './submit.ts'
import { loadDemoWallet, peekDemoWallet, resetDemoWallet, type KeyValueStorage } from './wallet.ts'

describe('validateUsername', () => {
  it('accepts normal names, including exactly 16 bytes', () => {
    expect(validateUsername('Ogata Koichi G')).toBeNull()
    expect(validateUsername('a'.repeat(MAX_USERNAME_BYTES))).toBeNull()
  })

  it('rejects blank names', () => {
    expect(validateUsername('')).not.toBeNull()
    expect(validateUsername('   ')).not.toBeNull()
  })

  it('counts bytes, not characters', () => {
    expect(validateUsername('a'.repeat(MAX_USERNAME_BYTES + 1))).toMatch(/17 of 16 bytes/)
    // 6 Japanese characters are 18 bytes in UTF-8, although only 6 characters long.
    expect(usernameBytes('岩瀬仁紀大野')).toBe(18)
    expect(validateUsername('岩瀬仁紀大野')).not.toBeNull()
    expect(validateUsername('岩瀬仁紀')).toBeNull()
  })
})

describe('isValidOrder', () => {
  it('requires each team exactly once', () => {
    expect(isValidOrder([0, 1, 2, 3, 4, 5])).toBe(true)
    expect(isValidOrder([5, 4, 3, 2, 1, 0])).toBe(true)
    expect(isValidOrder([0, 1, 2, 3, 4, 4])).toBe(false)
    expect(isValidOrder([0, 1, 2, 3, 4, 6])).toBe(false)
    expect(isValidOrder([0, 1, 2, 3, 4])).toBe(false)
    expect(isValidOrder([0, 1, 2, 3, 4, 5.5])).toBe(false)
  })
})

describe('moveTeam', () => {
  const order = [0, 1, 2, 3, 4, 5]

  it('swaps with the neighbour and keeps a valid order', () => {
    expect(moveTeam(order, 2, -1)).toEqual([0, 2, 1, 3, 4, 5])
    expect(moveTeam(order, 2, 1)).toEqual([0, 1, 3, 2, 4, 5])
    expect(isValidOrder(moveTeam(order, 2, 1))).toBe(true)
  })

  it('does nothing at the edges and never changes the input', () => {
    expect(moveTeam(order, 0, -1)).toEqual(order)
    expect(moveTeam(order, 5, 1)).toEqual(order)
    expect(order).toEqual([0, 1, 2, 3, 4, 5])
  })
})

describe('instruction encoding', () => {
  const discriminator = idl.instructions.find((i) => i.name === 'submit_prediction')!.discriminator

  it('writes discriminator, then a length-prefixed username, then the order', () => {
    const data = encodeSubmitData('Ab', [1, 0, 2, 3, 4, 5])
    expect([...data]).toEqual([...discriminator, 2, 0, 0, 0, 65, 98, 1, 0, 2, 3, 4, 5])
  })

  it('length-prefixes UTF-8 bytes, not characters', () => {
    const data = encodeSubmitData('岩', [0, 1, 2, 3, 4, 5])
    expect([...data.subarray(8, 12)]).toEqual([3, 0, 0, 0])
    expect(data.length).toBe(8 + 4 + 3 + 6)
  })

  it('lists the accounts in the order the IDL declares them', () => {
    const user = Keypair.generate().publicKey
    const ix = buildSubmitInstruction(user, 2026, 'Ab', [0, 1, 2, 3, 4, 5])
    expect(ix.programId.equals(PROGRAM_ID)).toBe(true)
    expect(ix.keys.map((k) => k.pubkey.toBase58())).toEqual([
      user.toBase58(),
      seasonAddress(2026).toBase58(),
      predictionAddress(2026, user).toBase58(),
      SystemProgram.programId.toBase58(),
    ])
    expect(ix.keys.map((k) => [k.isSigner, k.isWritable])).toEqual([
      [true, true],
      [false, false],
      [false, true],
      [false, false],
    ])
    // Same account order as the IDL.
    const idlAccounts = idl.instructions.find((i) => i.name === 'submit_prediction')!.accounts.map((a) => a.name)
    expect(idlAccounts).toEqual(['user', 'season', 'prediction', 'system_program'])
  })

  it('gives each wallet and each season its own prediction address', () => {
    const a = Keypair.generate().publicKey
    const b = Keypair.generate().publicKey
    expect(predictionAddress(2026, a).equals(predictionAddress(2026, a))).toBe(true)
    expect(predictionAddress(2026, a).equals(predictionAddress(2026, b))).toBe(false)
    expect(predictionAddress(2026, a).equals(predictionAddress(2027, a))).toBe(false)
  })
})

describe('describeError', () => {
  it('maps program error codes to the messages in the IDL', () => {
    // 6000 = 0x1770 DeadlinePassed, 6004 = 0x1774 InvalidUsername
    expect(describeError(new Error('Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1770'))).toBe(
      'The prediction deadline has passed',
    )
    expect(describeError(new Error('custom program error: 0x1774'))).toMatch(/Username must be 1 to 16 bytes/)
  })

  it('explains a second submission from the same wallet', () => {
    expect(describeError(new Error('Allocate: account Address { address: X } already in use'))).toMatch(
      /already submitted/,
    )
  })

  it('explains an empty wallet and an unreachable validator', () => {
    expect(describeError(new Error('Attempt to debit an account but found no record of a prior credit.'))).toMatch(
      /Get test SOL/,
    )
    expect(describeError(new TypeError('Failed to fetch'))).toMatch(/Is it running/)
  })

  it('passes unknown errors through', () => {
    expect(describeError(new Error('something else'))).toBe('something else')
    expect(describeError('plain string')).toBe('plain string')
  })
})

describe('demo wallet storage', () => {
  function fakeStorage(): KeyValueStorage & { data: Map<string, string> } {
    const data = new Map<string, string>()
    return {
      data,
      getItem: (k) => data.get(k) ?? null,
      setItem: (k, v) => void data.set(k, v),
      removeItem: (k) => void data.delete(k),
    }
  }

  it('creates a wallet once and then loads the same one', () => {
    const storage = fakeStorage()
    const first = loadDemoWallet(storage)
    const again = loadDemoWallet(storage)
    expect(again.publicKey.equals(first.publicKey)).toBe(true)
  })

  it('reset makes a different wallet and stores it', () => {
    const storage = fakeStorage()
    const first = loadDemoWallet(storage)
    const fresh = resetDemoWallet(storage)
    expect(fresh.publicKey.equals(first.publicKey)).toBe(false)
    expect(loadDemoWallet(storage).publicKey.equals(fresh.publicKey)).toBe(true)
  })

  it('peek returns the saved wallet but never creates one', () => {
    const storage = fakeStorage()
    expect(peekDemoWallet(storage)).toBeNull()
    expect(storage.data.size).toBe(0)

    const wallet = loadDemoWallet(storage)
    expect(peekDemoWallet(storage)?.publicKey.equals(wallet.publicKey)).toBe(true)

    storage.setItem('prediction-league/demo-wallet', 'not json')
    expect(peekDemoWallet(storage)).toBeNull()
  })

  it('replaces a corrupt entry instead of crashing', () => {
    const storage = fakeStorage()
    storage.setItem('prediction-league/demo-wallet', 'not json')
    expect(() => loadDemoWallet(storage)).not.toThrow()
    storage.setItem('prediction-league/demo-wallet', JSON.stringify([1, 2, 3]))
    expect(loadDemoWallet(storage).secretKey).toHaveLength(64)
  })
})
