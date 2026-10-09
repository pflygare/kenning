import { useEffect, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { InvitePreview } from '../api/types/InvitePreview'
import type { Membership } from '../api/types/Membership'
import { useAuth } from '../auth/context'
import { useAction } from '../hooks/useAction'

export default function Invite() {
  const { token = '' } = useParams()
  const { me, refresh, logout } = useAuth()
  const navigate = useNavigate()
  const [invite, setInvite] = useState<InvitePreview | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const { busy, error, run } = useAction()

  useEffect(() => {
    apiGet<InvitePreview>(`/invites/${encodeURIComponent(token)}`)
      .then(setInvite)
      .catch((err: unknown) => setLoadError(errorMessage(err)))
  }, [token])

  if (loadError) {
    return (
      <>
        <h1>Invitation not found</h1>
        <p>Check that you opened the whole link from your email.</p>
      </>
    )
  }
  if (!invite) {
    return <p className="status">Loading invitation…</p>
  }

  const heading = (
    <h1>
      Join {invite.org_name} on Kenning
    </h1>
  )
  const here = `/invite/${token}`

  if (invite.status !== 'open') {
    const reason = {
      accepted: 'has already been accepted',
      revoked: 'was withdrawn',
      expired: 'has expired',
    }[invite.status]
    return (
      <>
        {heading}
        <p>
          This invitation {reason}. Ask {invite.invited_by_name} to send a new one if you still need
          access.
        </p>
        {me && <Link to="/">Go to Kenning</Link>}
      </>
    )
  }

  const intro = (
    <p>
      {invite.invited_by_name} invited <strong>{invite.email}</strong> to join {invite.org_name} as{' '}
      {invite.role === 'admin' ? 'an admin' : `a ${invite.role}`}.
    </p>
  )

  if (!me) {
    const query = new URLSearchParams({ next: here, email: invite.email, invite: token })
    return (
      <>
        {heading}
        {intro}
        <div className="row">
          {invite.account_exists ? (
            <Link className="button" to={`/login?${query}`}>
              Sign in to accept
            </Link>
          ) : (
            <Link className="button" to={`/signup?${query}`}>
              Create an account to accept
            </Link>
          )}
        </div>
      </>
    )
  }

  if (me.user.email !== invite.email) {
    return (
      <>
        {heading}
        {intro}
        <p>
          You are signed in as {me.user.email}. Sign out and sign in as {invite.email} to accept.
        </p>
        <button onClick={() => void logout()}>Sign out</button>
      </>
    )
  }

  const accept = () =>
    run(async () => {
      const membership = await apiPost<Membership>(`/invites/${encodeURIComponent(token)}/accept`)
      await refresh()
      navigate(`/${membership.slug}`, { replace: true })
    })

  return (
    <>
      {heading}
      {intro}
      {error && <p className="error">{error}</p>}
      <button className="primary" onClick={() => void accept()} disabled={busy}>
        Join {invite.org_name}
      </button>
    </>
  )
}
