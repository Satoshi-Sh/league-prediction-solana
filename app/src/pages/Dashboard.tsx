import { useMemo, useState } from 'react'
import { DaySelector } from '../components/DaySelector.tsx'
import { Leaderboard } from '../components/Leaderboard.tsx'
import { ScoreChart } from '../components/ScoreChart.tsx'
import { StandingsPanel } from '../components/StandingsPanel.tsx'
import { POLL_MS, RPC_URL, SEASON_ID } from '../config.ts'
import { useLeague } from '../hooks/useLeague.ts'
import { buildBoards } from '../lib/ranking.ts'
import { predictionAddress } from '../lib/submit.ts'
import { peekDemoWallet } from '../lib/wallet.ts'

function Notice({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="card notice">
      <h2>{title}</h2>
      {children}
    </div>
  )
}

export default function Dashboard() {
  const [autoRefresh, setAutoRefresh] = useState(true)
  const { status, league, error, refreshing, refresh } = useLeague(RPC_URL, SEASON_ID, autoRefresh, POLL_MS)

  // null = follow the latest day, so new days appear by themselves while auto-refresh is on.
  const [chosenPosition, setChosenPosition] = useState<number | null>(null)
  // The prediction made from this browser's demo wallet, if there is one.
  const [mineAddress] = useState(() => {
    const wallet = peekDemoWallet(window.localStorage)
    return wallet ? predictionAddress(SEASON_ID, wallet.publicKey).toBase58() : undefined
  })
  const [pinned, setPinned] = useState<ReadonlySet<string>>(new Set())

  const boards = useMemo(
    () => (league ? buildBoards(league.predictions, league.days) : []),
    [league],
  )

  const togglePin = (address: string) =>
    setPinned((current) => {
      const next = new Set(current)
      if (!next.delete(address)) next.add(address)
      return next
    })

  const latest = boards.length - 1
  const position = chosenPosition === null ? latest : Math.min(chosenPosition, latest)
  const board = boards[position]

  return (
    <>
      <div className="toolbar">
        <p className="muted">
          Score 100 means a prediction matches the standings exactly; each place off costs points.
        </p>
        <div className="controls">
          {league && (
            <span className={`status ${league.season.resultsPosted ? 'final' : 'live'}`}>
              {league.season.resultsPosted ? 'Season final' : 'In progress'}
            </span>
          )}
          <label className="check">
            <input type="checkbox" checked={autoRefresh} onChange={(e) => setAutoRefresh(e.target.checked)} />
            Auto-refresh
          </label>
          <button type="button" onClick={refresh} disabled={refreshing}>
            {refreshing ? 'Refreshing…' : 'Refresh'}
          </button>
        </div>
      </div>

      {status === 'ready' && error && (
        <div className="warning">Could not refresh ({error}). Showing the last data that loaded.</div>
      )}

      {status === 'loading' && <Notice title="Loading…">Reading the season from {RPC_URL}.</Notice>}

      {status === 'error' && (
        <Notice title="Could not reach the cluster">
          <p>
            <code>{RPC_URL}</code> did not answer: {error}
          </p>
          <p className="muted">
            For the local demo, start the validator and run the seeder (see the seeder's <code>--help</code>),
            or set <code>VITE_RPC_URL</code> in <code>app/.env.local</code>.
          </p>
        </Notice>
      )}

      {status === 'empty' && (
        <Notice title={`Season ${SEASON_ID} was not found`}>
          <p>
            There is no season {SEASON_ID} on <code>{RPC_URL}</code> yet. Run the seeder to create it.
          </p>
        </Notice>
      )}

      {status === 'ready' && league && boards.length === 0 && (
        <Notice title="No results yet">
          <p>
            {league.predictions.length} predictions are in, but no daily results have been posted. The
            leaderboard appears once the first day is on-chain.
          </p>
        </Notice>
      )}

      {status === 'ready' && league && board && (
        <>
          <div className="summary">
            <span>
              <strong>{league.predictions.length}</strong> predictions
            </span>
            <span>
              <strong>{league.days.length}</strong> game days on-chain
            </span>
            <span className="muted">Updated {league.fetchedAt.toLocaleTimeString()}</span>
          </div>

          <div className="card">
            <DaySelector days={boards} position={position} onChange={setChosenPosition} />
          </div>

          <div className="grid">
            <Leaderboard rows={board.rows} pinned={pinned} onTogglePin={togglePin} mineAddress={mineAddress} />
            <div className="side">
              <StandingsPanel board={board} />
              <ScoreChart
                boards={boards}
                position={position}
                pinned={pinned}
                onClearPinned={() => setPinned(new Set())}
              />
            </div>
          </div>
        </>
      )}
    </>
  )
}
