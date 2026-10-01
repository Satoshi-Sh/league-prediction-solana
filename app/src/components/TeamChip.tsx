import { team } from '../lib/teams.ts'

/** Small coloured pill with the NPB team code (G, T, DB, C, D, S). */
export function TeamChip({ index }: { index: number }) {
  const t = team(index)
  return (
    <span className="chip" style={{ background: t.color, color: t.textColor }} title={t.name}>
      {t.code}
    </span>
  )
}

/** A predicted order as a row of chips, best team first. */
export function OrderChips({ order }: { order: readonly number[] }) {
  return (
    <span className="order" aria-label={order.map((i) => team(i).short).join(', ')}>
      {order.map((teamIndex, rank) => (
        <TeamChip key={rank} index={teamIndex} />
      ))}
    </span>
  )
}
