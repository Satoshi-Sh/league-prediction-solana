import { useRoute, ROUTE_HREF, type Route } from './hooks/useRoute.ts'
import Dashboard from './pages/Dashboard.tsx'
import Submit from './pages/Submit.tsx'

const TABS: { route: Route; label: string }[] = [
  { route: 'dashboard', label: 'Leaderboard' },
  { route: 'submit', label: 'Submit prediction' },
]

export default function App() {
  const route = useRoute()

  return (
    <div className="page">
      <header className="top">
        <div>
          <h1>Prediction League</h1>
          <p className="muted">2026 NPB Central League · predict the final order of the six teams.</p>
        </div>
        <nav className="tabs">
          {TABS.map((tab) => (
            <a
              key={tab.route}
              href={ROUTE_HREF[tab.route]}
              className={route === tab.route ? 'active' : undefined}
              aria-current={route === tab.route ? 'page' : undefined}
            >
              {tab.label}
            </a>
          ))}
        </nav>
      </header>

      {route === 'submit' ? <Submit /> : <Dashboard />}

      <footer>
        Predictions: pundit forecasts from{' '}
        <a href="https://www.ohtashp.com/topics/baseball_yosou/">ohtashp.com</a>. Results:{' '}
        <a href="https://npb.jp/bis/eng/">NPB official site</a>. Scores are computed on Solana by the{' '}
        <code>prediction_league</code> program.
      </footer>
    </div>
  )
}
