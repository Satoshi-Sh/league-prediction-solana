/** Central League teams. `index` is the team number used by the Solana program. */
export interface Team {
  index: number
  name: string
  short: string
  /** Abbreviation NPB uses on its site. */
  code: string
  color: string
  /** Readable text colour on top of `color`. */
  textColor: string
}

export const TEAMS: readonly Team[] = [
  { index: 0, name: 'Yomiuri Giants', short: 'Yomiuri', code: 'G', color: '#f26a1b', textColor: '#ffffff' },
  { index: 1, name: 'Hanshin Tigers', short: 'Hanshin', code: 'T', color: '#ffd400', textColor: '#1a1a1a' },
  { index: 2, name: 'YOKOHAMA DeNA BAYSTARS', short: 'DeNA', code: 'DB', color: '#2aa7e0', textColor: '#ffffff' },
  { index: 3, name: 'Hiroshima Toyo Carp', short: 'Hiroshima', code: 'C', color: '#d81e2c', textColor: '#ffffff' },
  { index: 4, name: 'Chunichi Dragons', short: 'Chunichi', code: 'D', color: '#1d3f8f', textColor: '#ffffff' },
  { index: 5, name: 'Tokyo Yakult Swallows', short: 'Yakult', code: 'S', color: '#2e9e4f', textColor: '#ffffff' },
]

export function team(index: number): Team {
  const t = TEAMS[index]
  if (!t) throw new Error(`unknown team index ${index}`)
  return t
}
