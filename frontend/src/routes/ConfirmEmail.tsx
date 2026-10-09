import { useState } from 'react'
import { apiPost } from '../api/client'
import { useAuth } from '../auth/context'
import { useAction } from '../hooks/useAction'

/** Shown instead of the app until the signed-in user confirms their email. */
export default function ConfirmEmail() {
  const { me, logout } = useAuth()
  const [resent, setResent] = useState(false)
  const { busy, error, run } = useAction()
  if (!me) return null

  const resend = () =>
    run(async () => {
      await apiPost('/auth/resend-verification')
      setResent(true)
    })

  return (
    <>
      <h1>Confirm your email</h1>
      <p>
        We sent a link to <strong>{me.user.email}</strong>. Open it to finish setting up your
        account.
      </p>
      {resent && <p className="notice">We sent a new link. The old one still works too.</p>}
      {error && <p className="error">{error}</p>}
      <div className="row">
        <button disabled={busy} onClick={() => void resend()}>
          Send the link again
        </button>
        <button className="link" onClick={() => void logout()}>
          Wrong address? Sign out
        </button>
      </div>
    </>
  )
}
