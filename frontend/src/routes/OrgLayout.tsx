import { useState } from 'react'
import { NavLink, Outlet, useLocation, useParams } from 'react-router'
import { useAuth } from '../auth/context'
import { TopicsContext } from '../topics/context'
import NewTopicForm from '../topics/NewTopicForm'
import TopicTree from '../topics/TopicTree'
import { useTopicList } from '../topics/useTopicList'
import NotFound from './NotFound'
import styles from './OrgLayout.module.css'

/** Resolves `/:org` to one of the user's organizations and frames its pages with a sidebar. */
export default function OrgLayout() {
  const { org: slug } = useParams()
  const { me } = useAuth()
  const { pathname } = useLocation()
  const org = me?.orgs.find((o) => o.slug === slug)
  const context = useTopicList(slug ?? '')
  const { topics } = context
  const [adding, setAdding] = useState(false)

  if (!org) {
    return <NotFound />
  }
  const link = ({ isActive }: { isActive: boolean }) =>
    isActive ? `${styles.link} ${styles.active}` : styles.link
  const home = `/${org.slug}`
  const onPages = pathname === home || pathname.startsWith(`${home}/p/`)

  return (
    <TopicsContext.Provider value={context}>
      <div className={styles.frame}>
        <nav className={styles.sidebar} aria-label={org.name}>
          <div className={styles.org}>
            <span className={styles.orgIcon}>{org.name[0]?.toUpperCase()}</span>
            <span className={styles.orgName}>{org.name}</span>
          </div>
          <NavLink to={home} className={() => link({ isActive: onPages })}>
            <HomeIcon />
            Pages
          </NavLink>
          <NavLink to={`${home}/drafts`} className={link}>
            <DraftIcon />
            My drafts
          </NavLink>
          <NavLink to={`${home}/templates`} className={link}>
            <TemplateIcon />
            Templates
          </NavLink>
          <NavLink to={`${home}/tags`} className={link}>
            <TagIcon />
            Tags
          </NavLink>
          <NavLink to={`${home}/categories`} className={link}>
            <ListIcon />
            Categories
          </NavLink>
          <NavLink to={`${home}/archive`} className={link}>
            <ArchiveIcon />
            Archive
          </NavLink>
          <div className={`${styles.section} ${styles.sectionRow}`}>
            Topics
            <button
              type="button"
              className={styles.add}
              aria-label="New topic"
              title="New topic"
              onClick={() => setAdding(true)}
            >
              +
            </button>
          </div>
          <div className={styles.topics}>
            {topics && <TopicTree org={org.slug} topics={topics} />}
            {adding && (
              <NewTopicForm org={org.slug} parentId={null} compact onClose={() => setAdding(false)} />
            )}
            {topics?.length === 0 && !adding && (
              <button type="button" className={styles.hint} onClick={() => setAdding(true)}>
                Group pages into topics
              </button>
            )}
          </div>
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
    </TopicsContext.Provider>
  )
}

function DraftIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M4 20h4L19 9l-4-4L4 16z" />
      <path d="m13.5 6.5 4 4" />
    </svg>
  )
}

function HomeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z" />
    </svg>
  )
}

function TagIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z" />
      <circle cx="7.5" cy="7.5" r="1.5" />
    </svg>
  )
}

function ListIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M7 9h2M7 13h2M7 17h2M12 9h5M12 13h5M12 17h5" />
    </svg>
  )
}

function TemplateIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="4" y="3" width="16" height="18" rx="2" strokeDasharray="3 2.2" />
      <path d="M8 8h8M8 12h8M8 16h5" />
    </svg>
  )
}

function ArchiveIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="4" width="18" height="5" rx="1" />
      <path d="M5 9v10a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V9M10 13h4" />
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
