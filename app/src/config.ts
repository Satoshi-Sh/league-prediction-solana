/** RPC endpoint. Defaults to a local validator; override with VITE_RPC_URL in app/.env.local. */
export const RPC_URL: string = import.meta.env.VITE_RPC_URL ?? 'http://127.0.0.1:8899'

/** Which season to show (the seeder creates season 2026 by default). */
export const SEASON_ID: number = Number(import.meta.env.VITE_SEASON_ID ?? 2026)

/** How often to re-read the chain when auto-refresh is on. */
export const POLL_MS = 15_000
