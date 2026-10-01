import { formatDate } from '../lib/ranking.ts'

interface Props {
  days: readonly { dayIndex: number; date: number }[]
  /** Position in `days` (0-based). */
  position: number
  onChange: (position: number) => void
}

export function DaySelector({ days, position, onChange }: Props) {
  const last = days.length - 1
  const current = days[position]

  return (
    <div className="day-selector">
      <button type="button" onClick={() => onChange(0)} disabled={position === 0} title="First day">
        ⏮
      </button>
      <button type="button" onClick={() => onChange(position - 1)} disabled={position === 0} title="Previous day">
        ◀
      </button>
      <input
        type="range"
        min={0}
        max={last}
        value={position}
        onChange={(e) => onChange(Number(e.target.value))}
        aria-label="Game day"
      />
      <button type="button" onClick={() => onChange(position + 1)} disabled={position === last} title="Next day">
        ▶
      </button>
      <button type="button" onClick={() => onChange(last)} disabled={position === last} title="Latest day">
        ⏭
      </button>
      <span className="day-label">
        <strong>{formatDate(current.date)}</strong>
        <small>
          day {position + 1} of {days.length}
        </small>
      </span>
    </div>
  )
}
