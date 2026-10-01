import { useMemo, useState } from 'react'
import { CartesianGrid, Legend, Line, LineChart, ReferenceLine, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import { formatDate, type DayBoard } from '../lib/ranking.ts'

const COLORS = ['#2563eb', '#dc2626', '#16a34a', '#d97706', '#7c3aed', '#0891b2', '#db2777', '#4b5563']
const DEFAULT_LINES = 5

type Metric = 'score' | 'rank'

interface Props {
  boards: readonly DayBoard[]
  /** Index in `boards` of the selected day. */
  position: number
  pinned: ReadonlySet<string>
  onClearPinned: () => void
}

export function ScoreChart({ boards, position, pinned, onClearPinned }: Props) {
  const [metric, setMetric] = useState<Metric>('score')
  const selected = boards[position]

  // Pinned users if any, otherwise whoever is top of the board on the selected day.
  const lines = useMemo(() => {
    const chosen = pinned.size > 0 ? selected.rows.filter((r) => pinned.has(r.address)) : selected.rows.slice(0, DEFAULT_LINES)
    return chosen.slice(0, COLORS.length)
  }, [pinned, selected])

  const data = useMemo(
    () =>
      boards.map((board) => {
        const point: Record<string, number> = { dayIndex: board.dayIndex, date: board.date }
        for (const line of lines) {
          const row = board.rows.find((r) => r.address === line.address)
          if (row) point[line.address] = row[metric]
        }
        return point
      }),
    [boards, lines, metric],
  )

  const maxRank = Math.max(...boards.flatMap((b) => b.rows.map((r) => r.rank)), 1)

  return (
    <section className="card">
      <div className="card-head">
        <h2>{pinned.size > 0 ? 'Pinned users' : `Top ${Math.min(DEFAULT_LINES, selected.rows.length)} on this day`}</h2>
        <div className="toggle" role="group" aria-label="Chart metric">
          <button type="button" className={metric === 'score' ? 'on' : ''} onClick={() => setMetric('score')}>
            Score
          </button>
          <button type="button" className={metric === 'rank' ? 'on' : ''} onClick={() => setMetric('rank')}>
            Rank
          </button>
        </div>
        {pinned.size > 0 && (
          <button type="button" className="link" onClick={onClearPinned}>
            Clear pins
          </button>
        )}
      </div>

      {lines.length === 0 ? (
        <p className="muted center">Nothing to chart yet.</p>
      ) : (
        <div className="chart">
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={data} margin={{ top: 8, right: 16, bottom: 0, left: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="dayIndex" tick={{ fontSize: 12 }} />
              <YAxis
                reversed={metric === 'rank'}
                domain={metric === 'score' ? [0, 100] : [1, maxRank]}
                allowDecimals={false}
                tick={{ fontSize: 12 }}
                width={36}
              />
              <Tooltip labelFormatter={(_, payload) => (payload?.[0] ? `${formatDate(payload[0].payload.date)}` : '')} />
              <Legend />
              <ReferenceLine x={selected.dayIndex} stroke="#9ca3af" strokeDasharray="4 2" />
              {lines.map((line, i) => (
                <Line
                  key={line.address}
                  dataKey={line.address}
                  name={line.username}
                  stroke={COLORS[i % COLORS.length]}
                  strokeWidth={2}
                  dot={false}
                  isAnimationActive={false}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </section>
  )
}
