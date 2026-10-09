import { createContext, useContext } from 'react'
import type { AuthConfig } from '../api/types/AuthConfig'
import type { Me } from '../api/types/Me'

export type Auth = {
  /** The signed-in user and their organizations, or `null` when signed out. */
  me: Me | null
  config: AuthConfig
  /** Replace the session state, such as after signing in. */
  setMe: (me: Me | null) => void
  /** Re-read the session from the server, such as after joining an org. */
  refresh: () => Promise<void>
  logout: () => Promise<void>
}

export const AuthContext = createContext<Auth | null>(null)

export function useAuth(): Auth {
  const auth = useContext(AuthContext)
  if (!auth) {
    throw new Error('useAuth must be used inside <AuthProvider>')
  }
  return auth
}

/** Where to go after signing in: `?next=` when it is a local path, else home. */
export function nextPath(search: URLSearchParams): string {
  const next = search.get('next')
  return next && next.startsWith('/') && !next.startsWith('//') && !next.includes('\\')
    ? next
    : '/'
}
