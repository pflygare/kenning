import { Fragment, useCallback, useEffect, useState } from 'react'
import { Link } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { OutboxEmail } from '../api/types/OutboxEmail'
import { useAuth } from '../auth/context'
import { useAction } from '../hooks/useAction'
import Page from '../components/Page'
import NotFound from './NotFound'

/** Shortcuts for trying Kenning without a mail server. Only on with `DEV_TOOLS`. */
export default function DevTools() {
  const { config } = useAuth()
  if (!config.dev_tools) {
    return <NotFound />
  }
  return (
    <Page>
      <div className="page-header">
        <div>
          <h1>Testing</h1>
          <p>Shortcuts for trying Kenning without a mail server.</p>
        </div>
      </div>
      <p className="notice">
        These tools are on because the server has <code>DEV_TOOLS=true</code>. Anyone can read
        every email here, so never turn it on where real people sign up.
      </p>
      <Account />
      <Outbox />
    </Page>
  )
}

function Account() {
  const { me, refresh } = useAuth()
  const { busy, error, run } = useAction()

  const confirm = () =>
    run(async () => {
      await apiPost('/dev/confirm-email')
      await refresh()
    })

  return (
    <section className="card">
      <h2>Your account</h2>
      {!me ? (
        <p className="card-description">
          <Link to="/login?next=/dev">Sign in</Link> to use the account shortcuts.
        </p>
      ) : me.user.email_verified ? (
        <p className="card-description">{me.user.email} is confirmed.</p>
      ) : (
        <div className="row">
          <span>{me.user.email} is not confirmed yet.</span>
          <button className="primary" disabled={busy} onClick={() => void confirm()}>
            Confirm my email
          </button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  )
}

function Outbox() {
  const [emails, setEmails] = useState<OutboxEmail[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(() => {
    apiGet<OutboxEmail[]>('/dev/emails')
      .then((emails) => {
        setEmails(emails)
        setError(null)
      })
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [])

  useEffect(load, [load])

  return (
    <section className="card">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <h2>Outbox</h2>
        <button className="small" onClick={load}>
          Refresh
        </button>
      </div>
      <p className="card-description">
        The latest 50 emails Kenning sent or would have sent, newest first.
      </p>
      {error && <p className="error">{error}</p>}
      {emails?.length === 0 && <p>No emails yet.</p>}
      {emails?.map((email, i) => (
        <article key={i} className="email">
          <div className="muted">
            To {email.to} · {new Date(email.sent_at).toLocaleString()}
          </div>
          <strong>{email.subject}</strong>
          <p className="email-body">
            <Linkified text={email.body} />
          </p>
        </article>
      ))}
    </section>
  )
}

/** Plain text with its http(s) URLs turned into links. */
function Linkified({ text }: { text: string }) {
  const parts = text.split(/(https?:\/\/\S+)/g)
  return parts.map((part, i) =>
    i % 2 === 1 ? (
      <a key={i} href={part}>
        {part}
      </a>
    ) : (
      <Fragment key={i}>{part}</Fragment>
    ),
  )
}
