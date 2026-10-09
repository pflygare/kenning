import { Link, Navigate } from 'react-router'
import { useAuth } from '../auth/context'
import { LogoMark } from '../components/Logo'
import Page from '../components/Page'
import ConfirmEmail from './ConfirmEmail'
import styles from './Home.module.css'

export default function Home() {
  const { me } = useAuth()

  if (!me) {
    return (
      <main className={styles.hero}>
        <LogoMark size={72} />
        <h1>Your team's knowledge, in one calm place</h1>
        <p>
          Kenning is a wiki for writing things down, organizing them into topics, and finding them
          again when you need them.
        </p>
        <div className="row">
          <Link className="button primary" to="/signup">
            Create a free account
          </Link>
          <Link className="button" to="/login">
            Sign in
          </Link>
        </div>
      </main>
    )
  }

  if (!me.user.email_verified) {
    return <ConfirmEmail />
  }

  if (me.orgs.length === 1) {
    return <Navigate to={`/${me.orgs[0].slug}`} replace />
  }

  return (
    <Page narrow>
      <div className="page-header">
        <div>
          <h1>Welcome, {me.user.name.split(' ')[0]}</h1>
          <p>
            {me.orgs.length === 0
              ? 'Create an organization for your team, or ask a colleague to invite you to theirs.'
              : 'Choose an organization to continue.'}
          </p>
        </div>
      </div>
      {me.orgs.length > 0 && (
        <ul className={styles.orgs}>
          {me.orgs.map((org) => (
            <li key={org.id}>
              <Link to={`/${org.slug}`} className={styles.org}>
                <span className={styles.orgIcon}>{org.name[0]?.toUpperCase()}</span>
                <span>
                  <span className={styles.orgName}>{org.name}</span>
                  <span className="muted">kenning/{org.slug}</span>
                </span>
                <span className="badge">{org.role}</span>
              </Link>
            </li>
          ))}
        </ul>
      )}
      <Link className={me.orgs.length ? 'button' : 'button primary'} to="/new-org">
        + Create an organization
      </Link>
    </Page>
  )
}
