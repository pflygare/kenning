import { useEffect, useState } from 'react'

type Health = { status: string; version: string }

function App() {
  const [health, setHealth] = useState<Health | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    fetch('/api/health')
      .then((res) => (res.ok ? res.json() : Promise.reject(new Error(`HTTP ${res.status}`))))
      .then(setHealth)
      .catch((err: Error) => setError(err.message))
  }, [])

  return (
    <main className="placeholder">
      <h1>Kenning</h1>
      <p>Pages, topics and search are coming soon.</p>
      <p className="status">
        Backend:{' '}
        {health ? `${health.status} (v${health.version})` : error ? `unreachable (${error})` : 'checking…'}
      </p>
    </main>
  )
}

export default App
