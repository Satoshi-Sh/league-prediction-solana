import { Keypair } from '@solana/web3.js'

const STORAGE_KEY = 'prediction-league/demo-wallet'

/** The part of `localStorage` used here, so tests can pass a plain object. */
export interface KeyValueStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
  removeItem(key: string): void
}

function readSaved(storage: KeyValueStorage): Keypair | null {
  const saved = storage.getItem(STORAGE_KEY)
  if (!saved) return null
  try {
    const bytes: unknown = JSON.parse(saved)
    if (Array.isArray(bytes) && bytes.length === 64) return Keypair.fromSecretKey(Uint8Array.from(bytes))
  } catch {
    // Corrupt entry: treat as missing.
  }
  return null
}

/** The saved demo wallet, or null. Unlike `loadDemoWallet` it never creates one. */
export function peekDemoWallet(storage: KeyValueStorage): Keypair | null {
  return readSaved(storage)
}

/**
 * A throwaway wallet for the demo. The secret key sits in the browser's localStorage so a page
 * reload keeps the same wallet (and the same prediction). Never put real funds in it.
 */
export function loadDemoWallet(storage: KeyValueStorage): Keypair {
  const existing = readSaved(storage)
  if (existing) return existing
  const wallet = Keypair.generate()
  storage.setItem(STORAGE_KEY, JSON.stringify(Array.from(wallet.secretKey)))
  return wallet
}

/** Forgets the current wallet and makes a new one, which can submit a new prediction. */
export function resetDemoWallet(storage: KeyValueStorage): Keypair {
  storage.removeItem(STORAGE_KEY)
  return loadDemoWallet(storage)
}
