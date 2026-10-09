import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router'
import { useAuth } from './context'

/** Sends signed-out visitors to the login page, then back here afterwards. */
export default function RequireAuth({ children }: { children: ReactNode }) {
  const { me } = useAuth()
  const location = useLocation()
  if (!me) {
    const next = location.pathname + location.search
    return <Navigate to={`/login?next=${encodeURIComponent(next)}`} replace />
  }
  return children
}
