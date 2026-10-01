import { useSyncExternalStore } from 'react'

export type Route = 'dashboard' | 'submit'

function current(): Route {
  return window.location.hash === '#/submit' ? 'submit' : 'dashboard'
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener('hashchange', onChange)
  return () => window.removeEventListener('hashchange', onChange)
}

/** Which page to show, from the URL hash: `#/submit` is the form, anything else the dashboard. */
export function useRoute(): Route {
  return useSyncExternalStore(subscribe, current)
}

export const ROUTE_HREF: Record<Route, string> = { dashboard: '#/', submit: '#/submit' }
