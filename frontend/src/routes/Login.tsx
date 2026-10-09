import { type FormEvent, useState } from 'react'
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router'
import { apiPost } from '../api/client'
import type { LoginRequest } from '../api/types/LoginRequest'
import type { Me } from '../api/types/Me'
import { nextPath, useAuth } from '../auth/context'
import GoogleButton from '../components/GoogleButton'
import { useAction } from '../hooks/useAction'

export default function Login() {
  const { me, config, setMe } = useAuth()
  const [search] = useSearchParams()
  const navigate = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const { busy, error, run } = useAction()
  const next = nextPath(search)

  if (me) {
    return <Navigate to={next} replace />
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: LoginRequest = { email, password }
      setMe(await apiPost<Me>('/auth/login', body))
      navigate(next, { replace: true })
    })
  }

  return (
    <>
      <h1>Sign in to Kenning</h1>
      {search.get('error') === 'google' && (
        <p className="error">Google sign-in did not complete. Please try again.</p>
      )}
      {config.google && (
        <>
          <GoogleButton next={next} />
          <p className="divider">or</p>
        </>
      )}
      <form className="form" onSubmit={submit}>
        <label>
          Email
          <input
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          Password
          <input
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {error && <p className="error">{error}</p>}
        <button className="primary" type="submit" disabled={busy}>
          Sign in
        </button>
      </form>
      <p className="muted">
        <Link to="/forgot-password">Forgot your password?</Link>
        <br />
        New here? <Link to={`/signup${search.size ? `?${search}` : ''}`}>Create an account</Link>
      </p>
    </>
  )
}
