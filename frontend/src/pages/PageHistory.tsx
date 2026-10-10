import { useEffect, useState } from 'react'
import { Link } from 'react-router'
import { apiGet, errorMessage } from '../api/client'
import type { RevisionSummary } from '../api/types/RevisionSummary'
import { dateTime, pagePath, restoredNote } from './format'
import styles from './PageHistory.module.css'
import RevisionBadges from './RevisionBadges'
import { usePage } from './usePage'

/** Every version of the page, newest first. */
export default function PageHistory() {
  const { org, api, page, error } = usePage()
  const [revisions, setRevisions] = useState<RevisionSummary[] | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<RevisionSummary[]>(`${api}/revisions`)
      .then(setRevisions)
      .catch((err: unknown) => setLoadError(errorMessage(err)))
  }, [api])

  if (error || loadError) return <p className="alert error">{error ?? loadError}</p>
  if (!page || !revisions) return <p className="status">Loading…</p>
  const here = pagePath(org.slug, page)
  const title = (page.draft ?? page.published)!.title

  return (
    <div className={styles.history}>
      <div className="page-header">
        <div>
          <h1>History</h1>
          <p>
            Every version of <Link to={here}>{title}</Link>. Open one to see what changed or to
            bring it back.
          </p>
        </div>
        <Link className="button" to={here}>
          Back to the page
        </Link>
      </div>
      <ol className={`card ${styles.list}`}>
        {revisions.map((revision) => {
          const note = restoredNote(revision)
          return (
            <li key={revision.revision_id}>
              <Link to={`${here}/history/${revision.revision_id}`} className={styles.item}>
                <span className={styles.when}>{dateTime(revision.updated_at)}</span>
                <RevisionBadges revision={revision} />
                <span className={`muted ${styles.who}`}>{revision.author_name}</span>
              </Link>
              {(revision.title !== title || note) && (
                <p className={`muted ${styles.detail}`}>
                  {revision.title !== title && <>Titled “{revision.title}”. </>}
                  {note}
                </p>
              )}
            </li>
          )
        })}
      </ol>
    </div>
  )
}
