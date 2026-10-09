import { useEffect, useRef, useState } from 'react'
import { Link, useSearchParams } from 'react-router'
import { apiPost, errorMessage } from '../api/client'
import type { TokenRequest } from '../api/types/TokenRequest'
import { useAuth } from '../auth/context'

export default function VerifyEmail() {
  const { me, refresh } = useAuth()
  const [search] = useSearchParams()
  const token = search.get('token') ?? ''
  const [state, setState] = useState<'checking' | 'done' | string>('checking')
  // Tokens work once, so don't send it twice when React re-runs effects.
  const sent = useRef(false)

  useEffect(() => {
    if (sent.current) return
    sent.current = true
    const body: TokenRequest = { token }
    apiPost('/auth/verify-email', body)
      .then(() => refresh())
      .then(() => setState('done'))
      .catch((err: unknown) => setState(errorMessage(err)))
  }, [token, refresh])

  if (state === 'checking') {
    return <p className="status">Confirming your email…</p>
  }
  if (state === 'done') {
    return (
      <>
        <h1>Email confirmed</h1>
        <p>
          {me ? <Link to="/">Continue to Kenning</Link> : <Link to="/login">Sign in to continue</Link>}
        </p>
      </>
    )
  }
  return (
    <>
      <h1>We could not confirm your email</h1>
      <p className="error">{state}</p>
      <p>
        {me ? <Link to="/">Send a new link</Link> : <Link to="/login">Sign in</Link>}
      </p>
    </>
  )
}
