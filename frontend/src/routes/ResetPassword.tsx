import { type FormEvent, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { apiPost } from '../api/client'
import type { Me } from '../api/types/Me'
import type { ResetPasswordRequest } from '../api/types/ResetPasswordRequest'
import { useAuth } from '../auth/context'
import { useAction } from '../hooks/useAction'

export default function ResetPassword() {
  const { setMe } = useAuth()
  const [search] = useSearchParams()
  const navigate = useNavigate()
  const token = search.get('token') ?? ''
  const [password, setPassword] = useState('')
  const { busy, error, run } = useAction()

  if (!token) {
    return (
      <>
        <h1>Link incomplete</h1>
        <p>
          Open the link from your email again, or <Link to="/forgot-password">ask for a new one</Link>.
        </p>
      </>
    )
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: ResetPasswordRequest = { token, password }
      setMe(await apiPost<Me>('/auth/reset-password', body))
      navigate('/', { replace: true })
    })
  }

  return (
    <>
      <h1>Choose a new password</h1>
      <p className="muted">This signs you out everywhere else.</p>
      <form className="form" onSubmit={submit}>
        <label>
          New password <span className="hint">At least 8 characters.</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        {error && (
          <p className="error">
            {error} <Link to="/forgot-password">Send a new link</Link>
          </p>
        )}
        <button className="primary" type="submit" disabled={busy}>
          Save password
        </button>
      </form>
    </>
  )
}
