import { useState } from 'react'
import { apiPost } from '../api/client'
import { useAuth } from '../auth/context'
import Page from '../components/Page'
import { useAction } from '../hooks/useAction'

/** Shown instead of the app until the signed-in user confirms their email. */
export default function ConfirmEmail() {
  const { me, config, refresh, logout } = useAuth()
  const [resent, setResent] = useState(false)
  const { busy, error, run } = useAction()
  if (!me) return null

  const resend = () =>
    run(async () => {
      await apiPost('/auth/resend-verification')
      setResent(true)
    })

  // Testing shortcut, only when the server has DEV_TOOLS on.
  const confirmNow = () =>
    run(async () => {
      await apiPost('/dev/confirm-email')
      await refresh()
    })

  return (
    <Page narrow>
      <div className="card">
        <h1>Confirm your email</h1>
        <p>
          We sent a link to <strong>{me.user.email}</strong>. Open it to finish setting up your
          account.
        </p>
        {resent && <p className="notice">We sent a new link. The old one still works too.</p>}
        {error && <p className="alert error">{error}</p>}
        <div className="row">
          <button disabled={busy} onClick={() => void resend()}>
            Send the link again
          </button>
          {config.dev_tools && (
            <button disabled={busy} onClick={() => void confirmNow()}>
              Testing: confirm without the email
            </button>
          )}
        </div>
        <p className="muted" style={{ margin: '1.25rem 0 0' }}>
          Wrong address?{' '}
          <button className="link" onClick={() => void logout()}>
            Sign out
          </button>{' '}
          and sign up again.
        </p>
      </div>
    </Page>
  )
}
