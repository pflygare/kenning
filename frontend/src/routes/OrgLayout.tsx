import { NavLink, Outlet, useParams } from 'react-router'
import { useAuth } from '../auth/context'
import NotFound from './NotFound'
import styles from './OrgLayout.module.css'

/** Resolves `/:org` to one of the user's organizations and frames its pages with a sidebar. */
export default function OrgLayout() {
  const { org: slug } = useParams()
  const { me } = useAuth()
  const org = me?.orgs.find((o) => o.slug === slug)
  if (!org) {
    return <NotFound />
  }
  const link = ({ isActive }: { isActive: boolean }) =>
    isActive ? `${styles.link} ${styles.active}` : styles.link

  return (
    <div className={styles.frame}>
      <nav className={styles.sidebar} aria-label={org.name}>
        <div className={styles.org}>
          <span className={styles.orgIcon}>{org.name[0]?.toUpperCase()}</span>
          <span className={styles.orgName}>{org.name}</span>
        </div>
        <NavLink to={`/${org.slug}`} end className={link}>
          <HomeIcon />
          Home
        </NavLink>
        <div className={styles.section}>Settings</div>
        <NavLink to={`/${org.slug}/settings/members`} className={link}>
          <PeopleIcon />
          Members
        </NavLink>
      </nav>
      <main className={styles.content}>
        <Outlet context={org} />
      </main>
    </div>
  )
}

function HomeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z" />
    </svg>
  )
}

function PeopleIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="9" cy="8" r="3.5" />
      <path d="M2.5 20a6.5 6.5 0 0 1 13 0" />
      <path d="M16 4.5a3.5 3.5 0 0 1 0 7M18 14.5a6.5 6.5 0 0 1 3.5 5.5" />
    </svg>
  )
}
