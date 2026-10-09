import { type FormEvent, useState } from 'react'
import { Link } from 'react-router'
import { apiPost } from '../api/client'
import type { ForgotPasswordRequest } from '../api/types/ForgotPasswordRequest'
import { useAction } from '../hooks/useAction'

export default function ForgotPassword() {
  const [email, setEmail] = useState('')
  const [sent, setSent] = useState(false)
  const { busy, error, run } = useAction()

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: ForgotPasswordRequest = { email }
      await apiPost('/auth/forgot-password', body)
      setSent(true)
    })
  }

  if (sent) {
    return (
      <>
        <h1>Check your email</h1>
        <p>
          If an account uses {email}, we have sent it a link to choose a new password. The link
          works for one hour.
        </p>
        <p className="footer">
          <Link to="/login">Back to sign in</Link>
        </p>
      </>
    )
  }

  return (
    <>
      <h1>Reset your password</h1>
      <p>Enter your email and we will send you a link to choose a new password.</p>
      <form className="form" onSubmit={submit}>
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
        {error && <p className="error">{error}</p>}
        <button className="primary block" type="submit" disabled={busy}>
          Send reset link
        </button>
      </form>
      <p className="footer">
        Remembered it? <Link to="/login">Sign in</Link>
      </p>
    </>
  )
}
