import { Connection, LAMPORTS_PER_SOL } from '@solana/web3.js'
import { useEffect, useMemo, useState, type FormEvent } from 'react'
import { OrderChips, TeamChip } from '../components/TeamChip.tsx'
import { RPC_URL, SEASON_ID } from '../config.ts'
import type { PredictionAccount, Season } from '../lib/accounts.ts'
import { fetchSeason } from '../lib/chain.ts'
import { TEAMS, team } from '../lib/teams.ts'
import {
  describeError,
  fetchMyPrediction,
  MAX_USERNAME_BYTES,
  moveTeam,
  requestTestSol,
  submitPrediction,
  usernameBytes,
  validateUsername,
} from '../lib/submit.ts'
import { loadDemoWallet, resetDemoWallet } from '../lib/wallet.ts'

const DEFAULT_ORDER = TEAMS.map((t) => t.index)

interface ChainInfo {
  season: Season | null
  mine: PredictionAccount | null
  balance: number
  /** When this was read (ms), used to decide whether the window has closed. */
  readAt: number
}

function shortKey(key: string): string {
  return `${key.slice(0, 4)}…${key.slice(-4)}`
}

export default function Submit() {
  const connection = useMemo(() => new Connection(RPC_URL, 'confirmed'), [])
  const [wallet, setWallet] = useState(() => loadDemoWallet(window.localStorage))
  const address = wallet.publicKey.toBase58()

  const [info, setInfo] = useState<ChainInfo | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [tick, setTick] = useState(0)

  const [username, setUsername] = useState('')
  const [order, setOrder] = useState<number[]>(DEFAULT_ORDER)
  const [sending, setSending] = useState(false)
  const [airdropping, setAirdropping] = useState(false)
  const [signature, setSignature] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)

  // Reads the season, this wallet's prediction and its balance. Re-runs when `tick` changes.
  useEffect(() => {
    let stale = false
    Promise.all([
      fetchSeason(connection, SEASON_ID),
      fetchMyPrediction(connection, SEASON_ID, wallet.publicKey),
      connection.getBalance(wallet.publicKey),
    ])
      .then(([season, mine, balance]) => {
        if (stale) return
        setInfo({ season, mine, balance, readAt: Date.now() })
        setLoadError(null)
      })
      .catch((e: unknown) => {
        if (!stale) setLoadError(describeError(e))
      })
    return () => {
      stale = true
    }
  }, [connection, wallet, tick])

  const reload = () => setTick((t) => t + 1)

  const trimmed = username.trim()
  const nameProblem = username === '' ? null : validateUsername(trimmed)
  const season = info?.season ?? null
  const closed = info !== null && season !== null && info.readAt / 1000 >= season.deadline
  const canSubmit = !sending && season !== null && !closed && validateUsername(trimmed) === null

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    if (!canSubmit) return
    setSending(true)
    setActionError(null)
    try {
      setSignature(await submitPrediction(connection, wallet, SEASON_ID, trimmed, order))
      reload()
    } catch (e) {
      setActionError(describeError(e))
    } finally {
      setSending(false)
    }
  }

  async function onAirdrop() {
    setAirdropping(true)
    setActionError(null)
    try {
      await requestTestSol(connection, wallet.publicKey)
      reload()
    } catch (e) {
      setActionError(describeError(e))
    } finally {
      setAirdropping(false)
    }
  }

  function onNewWallet() {
    setWallet(resetDemoWallet(window.localStorage))
    setInfo(null)
    setSignature(null)
    setActionError(null)
  }

  return (
    <div className="submit-grid">
      <section className="card">
        <h2>Submit your prediction</h2>

        {loadError && (
          <p className="error">
            Could not read <code>{RPC_URL}</code>: {loadError}
          </p>
        )}
        {!info && !loadError && <p className="muted">Loading…</p>}

        {info && info.season === null && (
          <p className="error">
            Season {SEASON_ID} does not exist on <code>{RPC_URL}</code>. Run the seeder first.
          </p>
        )}

        {info?.season && (
          <p className={`muted window ${closed ? 'closed' : ''}`}>
            {closed ? 'Predictions closed on ' : 'Predictions are open until '}
            <strong>{new Date(info.season.deadline * 1000).toLocaleString()}</strong>.
          </p>
        )}

        {info?.mine ? (
          <div className="success">
            <p>
              <strong>{info.mine.username}</strong>, your prediction is on-chain.
            </p>
            <OrderChips order={info.mine.order} />
            {signature && (
              <p className="muted sig">
                Transaction <code>{shortKey(signature)}</code>
              </p>
            )}
            <p>
              <a href="#/">See it on the leaderboard →</a>
            </p>
            <p className="muted">
              Each wallet can submit once. Use “New wallet” to submit another prediction.
            </p>
          </div>
        ) : (
          info?.season && (
            <form onSubmit={onSubmit}>
              <label className="field">
                <span>Your name on the leaderboard</span>
                <input
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="e.g. Taro Y."
                  autoComplete="off"
                  disabled={closed || sending}
                  aria-invalid={nameProblem !== null}
                />
                <small className={nameProblem ? 'bad' : 'muted'}>
                  {nameProblem ?? `${usernameBytes(trimmed)} of ${MAX_USERNAME_BYTES} bytes. Names don't have to be unique.`}
                </small>
              </label>

              <div className="field">
                <span>Predicted final order, best team first</span>
                <ol className="order-editor">
                  {order.map((teamIndex, rank) => (
                    <li key={teamIndex}>
                      <span className="standings-rank">{rank + 1}</span>
                      <TeamChip index={teamIndex} />
                      <span className="standings-name">{team(teamIndex).name}</span>
                      <button
                        type="button"
                        onClick={() => setOrder(moveTeam(order, rank, -1))}
                        disabled={rank === 0 || closed || sending}
                        aria-label={`Move ${team(teamIndex).short} up`}
                      >
                        ▲
                      </button>
                      <button
                        type="button"
                        onClick={() => setOrder(moveTeam(order, rank, 1))}
                        disabled={rank === order.length - 1 || closed || sending}
                        aria-label={`Move ${team(teamIndex).short} down`}
                      >
                        ▼
                      </button>
                    </li>
                  ))}
                </ol>
                <button type="button" className="link" onClick={() => setOrder(DEFAULT_ORDER)}>
                  Reset order
                </button>
              </div>

              {closed && (
                <p className="error">
                  The window is closed, so the program will reject new predictions. For a demo, create the
                  season with a longer window: <code>seeder --deadline-minutes 525600</code> on a fresh
                  validator (<code>--reset</code>).
                </p>
              )}
              {actionError && <p className="error">{actionError}</p>}

              <button type="submit" className="primary" disabled={!canSubmit}>
                {sending ? 'Sending…' : 'Submit prediction'}
              </button>
            </form>
          )
        )}
      </section>

      <section className="card">
        <h2>Your demo wallet</h2>
        <p className="muted">
          A throwaway key kept in this browser, so there is nothing to install. Never put real funds in it.
        </p>
        <p className="address" title="Wallet address">
          <code>{address}</code>
        </p>
        <p>
          Balance:{' '}
          <strong>{info ? (info.balance / LAMPORTS_PER_SOL).toFixed(3) : '…'} SOL</strong>
        </p>
        <div className="row">
          <button type="button" onClick={onAirdrop} disabled={airdropping || sending}>
            {airdropping ? 'Requesting…' : 'Get test SOL'}
          </button>
          <button type="button" onClick={onNewWallet} disabled={airdropping || sending}>
            New wallet
          </button>
        </div>
        <p className="muted">
          Submitting asks for test SOL by itself when the wallet is nearly empty. The prediction account
          costs a little rent.
        </p>
        {info?.mine && actionError && <p className="error">{actionError}</p>}
      </section>
    </div>
  )
}
