import { type FormEvent, useState } from 'react'
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router'
import { apiPost } from '../api/client'
import type { Me } from '../api/types/Me'
import type { SignupRequest } from '../api/types/SignupRequest'
import { nextPath, useAuth } from '../auth/context'
import GoogleButton from '../components/GoogleButton'
import { useAction } from '../hooks/useAction'

export default function Signup() {
  const { me, config, setMe } = useAuth()
  const [search] = useSearchParams()
  const navigate = useNavigate()
  // Arriving from an invitation pre-fills the address it was sent to.
  const invite = search.get('invite') ?? undefined
  const [name, setName] = useState('')
  const [email, setEmail] = useState(search.get('email') ?? '')
  const [password, setPassword] = useState('')
  const { busy, error, run } = useAction()
  const next = nextPath(search)

  if (me) {
    return <Navigate to={next} replace />
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: SignupRequest = { name, email, password, invite_token: invite }
      setMe(await apiPost<Me>('/auth/signup', body))
      navigate(next, { replace: true })
    })
  }

  return (
    <>
      <h1>Create your account</h1>
      <p>Start writing things down with your team.</p>
      {config.google && (
        <>
          <GoogleButton next={next} />
          <p className="divider">or</p>
        </>
      )}
      <form className="form" onSubmit={submit}>
        <label>
          Name
          <input
            autoComplete="name"
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label>
          Email address
          <input
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          Password <span className="hint">At least 8 characters.</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {error && <p className="error">{error}</p>}
        <button className="primary block" type="submit" disabled={busy}>
          Create account
        </button>
      </form>
      <p className="footer">
        Already have an account? <Link to={`/login${search.size ? `?${search}` : ''}`}>Sign in</Link>
      </p>
    </>
  )
}
