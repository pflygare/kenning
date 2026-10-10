import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { ArchivedPage } from '../api/types/ArchivedPage'
import type { PageDetail } from '../api/types/PageDetail'
import { useAction } from '../hooks/useAction'
import { useOrg } from '../routes/useOrg'
import { pagePath, timeAgo } from './format'
import archive from './ArchivedPages.module.css'
import styles from './PageList.module.css'

/** Archived pages, which can be brought back as they were. */
export default function ArchivedPages() {
  const org = useOrg()
  const navigate = useNavigate()
  const action = useAction()
  const [pages, setPages] = useState<ArchivedPage[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<ArchivedPage[]>(`/orgs/${org.slug}/archive`)
      .then(setPages)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [org.slug])

  const restore = (page: ArchivedPage) =>
    action.run(async () => {
      const restored = await apiPost<PageDetail>(
        `/orgs/${org.slug}/pages/${page.short_id}/unarchive`,
      )
      navigate(pagePath(org.slug, restored))
    })

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Archive</h1>
          <p>Archived pages are hidden from lists and search. Restore one to bring it back.</p>
        </div>
      </div>
      {(error || action.error) && <p className="alert error">{error ?? action.error}</p>}
      {pages?.length === 0 && <p className="muted">Nothing is archived.</p>}
      {pages && pages.length > 0 && (
        <ul className={`card ${styles.list}`}>
          {pages.map((page) => (
            <li key={page.short_id} className={styles.row}>
              <span className={`${styles.title} ${archive.title}`}>{page.title}</span>
              <span className={`muted ${styles.meta}`}>
                Archived {page.archived_by_name ? `by ${page.archived_by_name} ` : ''}
                {timeAgo(page.archived_at)}
              </span>
              <button className="small" disabled={action.busy} onClick={() => void restore(page)}>
                Restore
              </button>
            </li>
          ))}
        </ul>
      )}
    </>
  )
}
