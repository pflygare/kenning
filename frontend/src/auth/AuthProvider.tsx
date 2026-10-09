import { type ReactNode, useCallback, useEffect, useMemo, useState } from 'react'
import { ApiError, apiGet, apiPost } from '../api/client'
import type { AuthConfig } from '../api/types/AuthConfig'
import type { Me } from '../api/types/Me'
import { AuthContext } from './context'

async function fetchMe(): Promise<Me | null> {
  try {
    return await apiGet<Me>('/auth/me')
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) {
      return null
    }
    throw err
  }
}

/** Loads the session once, then shares it with the whole app. */
export default function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null)
  const [config, setConfig] = useState<AuthConfig>({ google: false })
  const [loaded, setLoaded] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    Promise.all([fetchMe(), apiGet<AuthConfig>('/auth/config')])
      .then(([me, config]) => {
        setMe(me)
        setConfig(config)
        setLoaded(true)
      })
      .catch((err: Error) => setError(err.message))
  }, [])

  const refresh = useCallback(async () => setMe(await fetchMe()), [])
  const logout = useCallback(async () => {
    await apiPost('/auth/logout')
    setMe(null)
  }, [])

  const auth = useMemo(
    () => ({ me, config, setMe, refresh, logout }),
    [me, config, refresh, logout],
  )

  if (error) {
    return <p className="status">Kenning could not reach its server ({error}).</p>
  }
  if (!loaded) {
    return null
  }
  return <AuthContext.Provider value={auth}>{children}</AuthContext.Provider>
}
