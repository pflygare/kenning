import { Link, Outlet, useNavigate, useParams } from 'react-router'
import { useAuth } from '../auth/context'
import { Wordmark } from '../components/Logo'
import UserMenu from '../components/UserMenu'
import styles from './Layout.module.css'

const NEW_ORG = '__new__'

/** The signed-in app's frame: top bar with organization switcher and account menu. */
export default function Layout() {
  const { me, config } = useAuth()
  const { org: slug } = useParams()
  const navigate = useNavigate()
  const inOrg = me?.orgs.some((o) => o.slug === slug)

  return (
    <div className={styles.shell}>
      <header className={styles.header}>
        <Link to="/" className={styles.brand} aria-label="Kenning home">
          <Wordmark size={26} />
        </Link>
        {me && me.orgs.length > 0 && (
          <>
            <span className={styles.separator} aria-hidden="true">
              /
            </span>
            <select
              className={styles.orgs}
              aria-label="Organization"
              value={inOrg ? slug : ''}
              onChange={(e) =>
                navigate(e.target.value === NEW_ORG ? '/new-org' : `/${e.target.value}`)
              }
            >
              {!inOrg && <option value="">Choose organization</option>}
              {me.orgs.map((org) => (
                <option key={org.id} value={org.slug}>
                  {org.name}
                </option>
              ))}
              <option value={NEW_ORG}>+ New organization</option>
            </select>
          </>
        )}
        <span className={styles.spacer} />
        {config.dev_tools && (
          <Link to="/dev" className={styles.testing}>
            Testing
          </Link>
        )}
        {me ? (
          <UserMenu user={me.user} />
        ) : (
          <div className="row">
            <Link to="/login" className="button">
              Sign in
            </Link>
            <Link to="/signup" className="button primary">
              Get started
            </Link>
          </div>
        )}
      </header>
      <Outlet />
    </div>
  )
}
