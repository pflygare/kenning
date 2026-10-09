import { useEffect, useState } from 'react'
import { apiGet } from '../api/client'
import type { Health } from '../api/types/Health'

export default function Home() {
  const [health, setHealth] = useState<Health | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<Health>('/health')
      .then(setHealth)
      .catch((err: Error) => setError(err.message))
  }, [])

  return (
    <>
      <h1>Kenning</h1>
      <p>Pages, topics and search are coming soon.</p>
      <p className="status">
        Backend:{' '}
        {health
          ? `${health.status}, database ${health.database} (v${health.version})`
          : error
            ? `unreachable (${error})`
            : 'checking…'}
      </p>
    </>
  )
}
