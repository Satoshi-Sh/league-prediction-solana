import type { DayBoard } from '../lib/ranking.ts'
import { team } from '../lib/teams.ts'
import { MovementBadge } from './MovementBadge.tsx'
import { TeamChip } from './TeamChip.tsx'

/** The real league table for the selected day, with places gained or lost since the day before. */
export function StandingsPanel({ board }: { board: DayBoard }) {
  return (
    <section className="card">
      <h2>Central League standings</h2>
      <ol className="standings">
        {board.standings.map((teamIndex, rank) => (
          <li key={teamIndex}>
            <span className="standings-rank">{rank + 1}</span>
            <TeamChip index={teamIndex} />
            <span className="standings-name">{team(teamIndex).short}</span>
            <MovementBadge movement={board.teamMovement[teamIndex]} />
          </li>
        ))}
      </ol>
    </section>
  )
}
