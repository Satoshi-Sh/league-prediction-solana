import { useEffect, useMemo, useRef, useState } from 'react'
import type { BoardRow } from '../lib/ranking.ts'
import { MovementBadge } from './MovementBadge.tsx'
import { OrderChips } from './TeamChip.tsx'

const COLLAPSED_ROWS = 20

interface Props {
  rows: readonly BoardRow[]
  pinned: ReadonlySet<string>
  onTogglePin: (address: string) => void
  /** Prediction account of the person looking at the page: highlighted, and scrolled into view once. */
  mineAddress?: string
}

function scoreDeltaText(delta: number | null): string {
  if (delta === null || delta === 0) return ''
  return delta > 0 ? `+${delta}` : String(delta)
}

function rowClass(mine: boolean, pinned: boolean): string | undefined {
  return [mine && 'mine', pinned && 'pinned'].filter(Boolean).join(' ') || undefined
}

export function Leaderboard({ rows, pinned, onTogglePin, mineAddress }: Props) {
  const [query, setQuery] = useState('')
  const [expanded, setExpanded] = useState(false)

  // Scroll to your own row when the page opens, but not on every refresh or day change.
  const mineRow = useRef<HTMLTableRowElement | null>(null)
  const scrolled = useRef(false)
  useEffect(() => {
    if (scrolled.current || !mineRow.current) return
    scrolled.current = true
    mineRow.current.scrollIntoView({ behavior: 'smooth', block: 'center' })
  })

  // A rank is shared when another user has the same one.
  const sharedRanks = useMemo(() => {
    const counts = new Map<number, number>()
    for (const row of rows) counts.set(row.rank, (counts.get(row.rank) ?? 0) + 1)
    return new Set([...counts].filter(([, n]) => n > 1).map(([rank]) => rank))
  }, [rows])

  const needle = query.trim().toLowerCase()
  const filtered = needle ? rows.filter((r) => r.username.toLowerCase().includes(needle)) : rows
  // Pinned users and your own row stay visible even when the list is collapsed.
  const visible =
    needle || expanded
      ? filtered
      : filtered.filter((r, i) => i < COLLAPSED_ROWS || pinned.has(r.address) || r.address === mineAddress)
  const hidden = filtered.length - visible.length

  return (
    <section className="card">
      <div className="card-head">
        <h2>Leaderboard</h2>
        <input
          type="search"
          className="search"
          placeholder="Find a user…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Find a user"
        />
      </div>

      <div className="table-wrap">
        <table className="leaderboard">
          <thead>
            <tr>
              <th className="col-rank">Rank</th>
              <th className="col-move" />
              <th>User</th>
              <th>Predicted order</th>
              <th className="col-score">Score</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((row) => (
              <tr
                key={row.address}
                ref={row.address === mineAddress ? mineRow : undefined}
                className={rowClass(row.address === mineAddress, pinned.has(row.address))}
              >
                <td className="col-rank">
                  {row.rank}
                  {sharedRanks.has(row.rank) && <span title="Tied">=</span>}
                </td>
                <td className="col-move">
                  <MovementBadge movement={row.movement} />
                </td>
                <td>
                  <button
                    type="button"
                    className={`pin ${pinned.has(row.address) ? 'on' : ''}`}
                    onClick={() => onTogglePin(row.address)}
                    title={pinned.has(row.address) ? 'Remove from chart' : 'Add to chart'}
                    aria-pressed={pinned.has(row.address)}
                  >
                    {pinned.has(row.address) ? '★' : '☆'}
                  </button>
                  {row.username}
                  {row.address === mineAddress && <span className="you">You</span>}
                </td>
                <td>
                  <OrderChips order={row.order} />
                </td>
                <td className="col-score">
                  <span className="score">{row.score}</span>
                  <span className={`delta ${row.scoreDelta !== null && row.scoreDelta < 0 ? 'neg' : 'pos'}`}>
                    {scoreDeltaText(row.scoreDelta)}
                  </span>
                  <span className="bar" aria-hidden>
                    <span style={{ width: `${row.score}%` }} />
                  </span>
                </td>
              </tr>
            ))}
            {visible.length === 0 && (
              <tr>
                <td colSpan={5} className="muted center">
                  {rows.length === 0 ? 'No predictions have been submitted yet.' : 'No user matches that search.'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {(hidden > 0 || (expanded && !needle && filtered.length > COLLAPSED_ROWS)) && (
        <button type="button" className="link" onClick={() => setExpanded(!expanded)}>
          {expanded ? 'Show top users only' : `Show all (${hidden} more)`}
        </button>
      )}
    </section>
  )
}
