import { useCallback, useEffect, useState } from 'react'
import { fetchLeague, type League } from '../lib/chain.ts'

export type LeagueStatus = 'loading' | 'error' | 'empty' | 'ready'

export interface UseLeague {
  status: LeagueStatus
  league: League | null
  /** Set when the last read failed. With data already on screen it is shown as a warning. */
  error: string | null
  refreshing: boolean
  refresh: () => void
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

/** Reads the season from the chain now, and again every `pollMs` when `autoRefresh` is on. */
export function useLeague(
  rpcUrl: string,
  seasonId: number,
  autoRefresh: boolean,
  pollMs: number,
): UseLeague {
  // undefined = not loaded yet, null = season does not exist on this cluster.
  const [league, setLeague] = useState<League | null | undefined>(undefined)
  const [error, setError] = useState<string | null>(null)
  // True from the start: the first read begins as soon as the component mounts.
  const [refreshing, setRefreshing] = useState(true)
  // Bumping this re-runs the effect below, which is how manual and timed refreshes work.
  const [tick, setTick] = useState(0)

  useEffect(() => {
    let stale = false
    fetchLeague(rpcUrl, seasonId)
      .then((result) => {
        if (stale) return
        setLeague(result)
        setError(null)
      })
      .catch((e: unknown) => {
        if (!stale) setError(messageOf(e))
      })
      .finally(() => {
        if (!stale) setRefreshing(false)
      })
    return () => {
      stale = true
    }
  }, [rpcUrl, seasonId, tick])

  const reload = useCallback(() => {
    setRefreshing(true)
    setTick((t) => t + 1)
  }, [])

  useEffect(() => {
    if (!autoRefresh) return
    const id = setInterval(reload, pollMs)
    return () => clearInterval(id)
  }, [autoRefresh, reload, pollMs])

  const status: LeagueStatus =
    league === undefined ? (error ? 'error' : 'loading') : league === null ? 'empty' : 'ready'

  return { status, league: league ?? null, error, refreshing, refresh: reload }
}
