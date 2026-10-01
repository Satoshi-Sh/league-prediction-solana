interface Props {
  /** Positive = moved up, negative = moved down, 0 = unchanged, null = no previous day. */
  movement: number | null
  unit?: string
}

export function MovementBadge({ movement, unit = '' }: Props) {
  if (movement === null) return <span className="move move-none" title="No previous day">·</span>
  if (movement === 0) return <span className="move move-same" title="No change">–</span>
  const up = movement > 0
  return (
    <span
      className={`move ${up ? 'move-up' : 'move-down'}`}
      title={`${up ? 'Up' : 'Down'} ${Math.abs(movement)} since the previous day`}
    >
      {up ? '▲' : '▼'}
      {Math.abs(movement)}
      {unit}
    </span>
  )
}
