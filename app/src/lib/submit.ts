/**
 * Everything the "Submit prediction" page needs, apart from React: validating input, building the
 * `submit_prediction` instruction by hand (same approach as accounts.ts: no Anchor client), funding
 * the demo wallet, sending the transaction, and turning failures into readable messages.
 */
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js'
import { Buffer } from 'buffer'
import idl from '../idl/prediction_league.json'
import { decodePrediction, type PredictionAccount } from './accounts.ts'
import { PROGRAM_ID, seasonAddress } from './chain.ts'

/** Same limits as programs/prediction_league/src/validation.rs. */
export const MAX_USERNAME_BYTES = 16
const TEAM_COUNT = 6

/** The wallet needs this much to pay for the new account (rent) and the fee. */
const MIN_BALANCE = 0.01 * LAMPORTS_PER_SOL
const AIRDROP = LAMPORTS_PER_SOL

export function usernameBytes(username: string): number {
  return new TextEncoder().encode(username).length
}

/** null when the name is fine, otherwise a message for the user. Mirrors `is_valid_username`. */
export function validateUsername(username: string): string | null {
  if (username.trim() === '') return 'Enter a name.'
  const bytes = usernameBytes(username)
  if (bytes > MAX_USERNAME_BYTES) {
    return `Too long: ${bytes} of ${MAX_USERNAME_BYTES} bytes (characters outside A–Z take 2 to 4 bytes each).`
  }
  return null
}

/** True if every team 0..5 appears exactly once. Mirrors `is_valid_order`. */
export function isValidOrder(order: readonly number[]): boolean {
  return (
    order.length === TEAM_COUNT &&
    new Set(order).size === TEAM_COUNT &&
    order.every((team) => Number.isInteger(team) && team >= 0 && team < TEAM_COUNT)
  )
}

/** Swaps the team at `index` with its neighbour. Returns the same order at the edges. */
export function moveTeam(order: readonly number[], index: number, direction: -1 | 1): number[] {
  const target = index + direction
  const next = [...order]
  if (target < 0 || target >= next.length) return next
  ;[next[index], next[target]] = [next[target], next[index]]
  return next
}

export function predictionAddress(seasonId: number, user: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [new TextEncoder().encode('prediction'), seasonAddress(seasonId).toBytes(), user.toBytes()],
    PROGRAM_ID,
  )[0]
}

/** Borsh: discriminator, username as (u32 length + UTF-8 bytes), then the six team numbers. */
export function encodeSubmitData(username: string, order: readonly number[]): Buffer {
  const instruction = idl.instructions.find((i) => i.name === 'submit_prediction')
  if (!instruction) throw new Error('submit_prediction is not in the IDL')
  const name = new TextEncoder().encode(username)
  const length = Buffer.alloc(4)
  length.writeUInt32LE(name.length)
  return Buffer.concat([Buffer.from(instruction.discriminator), length, Buffer.from(name), Buffer.from(order)])
}

export function buildSubmitInstruction(
  user: PublicKey,
  seasonId: number,
  username: string,
  order: readonly number[],
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    // Same order as the accounts in the IDL.
    keys: [
      { pubkey: user, isSigner: true, isWritable: true },
      { pubkey: seasonAddress(seasonId), isSigner: false, isWritable: false },
      { pubkey: predictionAddress(seasonId, user), isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: encodeSubmitData(username, order),
  })
}

/** The wallet's prediction for this season, or null if it has not submitted one. */
export async function fetchMyPrediction(
  connection: Connection,
  seasonId: number,
  user: PublicKey,
): Promise<PredictionAccount | null> {
  const account = await connection.getAccountInfo(predictionAddress(seasonId, user))
  return account ? decodePrediction(account.data) : null
}

/** Tops the wallet up from the validator's faucet. Only works on local and test clusters. */
export async function requestTestSol(connection: Connection, user: PublicKey): Promise<void> {
  const signature = await connection.requestAirdrop(user, AIRDROP)
  const latest = await connection.getLatestBlockhash()
  await connection.confirmTransaction({ signature, ...latest }, 'confirmed')
}

/** Validates, funds the wallet if it is nearly empty, sends the transaction. Returns the signature. */
export async function submitPrediction(
  connection: Connection,
  wallet: Keypair,
  seasonId: number,
  username: string,
  order: readonly number[],
): Promise<string> {
  const problem = validateUsername(username)
  if (problem) throw new Error(problem)
  if (!isValidOrder(order)) throw new Error('Each team must appear exactly once.')

  if ((await connection.getBalance(wallet.publicKey)) < MIN_BALANCE) {
    await requestTestSol(connection, wallet.publicKey)
  }

  const transaction = new Transaction().add(buildSubmitInstruction(wallet.publicKey, seasonId, username, order))
  return sendAndConfirmTransaction(connection, transaction, [wallet], { commitment: 'confirmed' })
}

/** Turns a failed send into one sentence a person can act on. */
export function describeError(error: unknown): string {
  const text = error instanceof Error ? error.message : String(error)

  const custom = /custom program error: 0x([0-9a-f]+)/i.exec(text)
  if (custom) {
    const code = parseInt(custom[1], 16)
    const known = idl.errors.find((e) => e.code === code)
    if (known) return known.msg
  }
  if (/already in use/i.test(text)) return 'This wallet has already submitted a prediction for this season.'
  if (/no record of a prior credit|insufficient (funds|lamports)/i.test(text)) {
    return 'The wallet has no SOL to pay for the account. Use "Get test SOL" first.'
  }
  if (/failed to fetch|fetch failed|ECONNREFUSED/i.test(text)) {
    return 'Could not reach the validator. Is it running?'
  }
  if (/airdrop/i.test(text)) return `The airdrop failed (${text}). Airdrops only work on a local or test cluster.`
  return text
}
