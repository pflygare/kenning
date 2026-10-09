import { Link, Outlet, useNavigate, useParams } from 'react-router'
import { useAuth } from '../auth/context'
import styles from './Layout.module.css'

const NEW_ORG = '__new__'

export default function Layout() {
  const { me, config, logout } = useAuth()
  const { org: slug } = useParams()
  const navigate = useNavigate()

  const switchOrg = (value: string) => navigate(value === NEW_ORG ? '/new-org' : `/${value}`)

  return (
    <div className={styles.shell}>
      <header className={styles.header}>
        <Link to="/" className={styles.brand}>
          Kenning
        </Link>
        {me && me.orgs.length > 0 && (
          <select
            className={styles.orgs}
            aria-label="Organization"
            value={me.orgs.some((o) => o.slug === slug) ? slug : ''}
            onChange={(e) => switchOrg(e.target.value)}
          >
            {!me.orgs.some((o) => o.slug === slug) && <option value="">Choose organization</option>}
            {me.orgs.map((org) => (
              <option key={org.id} value={org.slug}>
                {org.name}
              </option>
            ))}
            <option value={NEW_ORG}>New organization…</option>
          </select>
        )}
        <span className={styles.spacer} />
        {config.dev_tools && <Link to="/dev">Testing</Link>}
        {me ? (
          <div className="row">
            <span className={styles.user} title={me.user.email}>
              {me.user.avatar_url && (
                <img className={styles.avatar} src={me.user.avatar_url} alt="" />
              )}
              {me.user.name}
            </span>
            {/* Protected pages send you to the login page on their own. */}
            <button onClick={() => void logout()}>Sign out</button>
          </div>
        ) : (
          <Link to="/login">Sign in</Link>
        )}
      </header>
      <main className={styles.main}>
        <Outlet />
      </main>
    </div>
  )
}
