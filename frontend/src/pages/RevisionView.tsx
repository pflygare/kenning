import { useEffect, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import type { RestoreRevisionRequest } from '../api/types/RestoreRevisionRequest'
import type { RevisionDetail } from '../api/types/RevisionDetail'
import type { RevisionSummary } from '../api/types/RevisionSummary'
import { useAction } from '../hooks/useAction'
import Diff, { Added, Removed } from './Diff'
import { dateTime, pagePath, restoredNote } from './format'
import MarkdownView from './MarkdownView'
import RevisionBadges from './RevisionBadges'
import styles from './RevisionView.module.css'
import { usePage } from './usePage'

type Mode = 'content' | 'changes' | 'current'

/** One old version: its content, what it changed, how it differs from now, and restoring it. */
export default function RevisionView() {
  const { org, api, page, error } = usePage()
  const { revision: id = '' } = useParams()
  const navigate = useNavigate()
  const action = useAction()
  const [detail, setDetail] = useState<RevisionDetail | null>(null)
  const [all, setAll] = useState<RevisionSummary[] | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('content')

  useEffect(() => {
    apiGet<RevisionDetail>(`${api}/revisions/${id}`)
      .then(setDetail)
      .catch((err: unknown) => setLoadError(errorMessage(err)))
  }, [api, id])

  useEffect(() => {
    apiGet<RevisionSummary[]>(`${api}/revisions`)
      .then(setAll)
      .catch(() => setAll([]))
  }, [api])

  if (error || loadError) return <p className="alert error">{error ?? loadError}</p>
  if (!page || detail?.revision.revision_id !== id) return <p className="status">Loading…</p>

  const here = pagePath(org.slug, page)
  const { revision } = detail
  const current = (page.draft ?? page.published)!
  const shown = { title: revision.title, body_md: detail.body_md }
  const index = all?.findIndex((r) => r.revision_id === revision.revision_id) ?? -1
  const newer = index > 0 ? all![index - 1] : null
  const older = index >= 0 ? all![index + 1] : null
  const note = restoredNote(revision)

  const restore = () => {
    const message = page.published
      ? 'Restore this version as a draft? Readers keep seeing the published version until you publish it.'
      : 'Restore this version? It replaces the draft, which stays in the history.'
    if (!window.confirm(message)) return
    void action.run(async () => {
      const body: RestoreRevisionRequest = { base_revision_id: current.revision_id }
      const restored = await apiPost<PageDetail>(`${api}/revisions/${id}/restore`, body)
      navigate(pagePath(org.slug, restored))
    })
  }

  return (
    <div className={styles.revision}>
      <nav className={styles.crumbs} aria-label="Page history">
        <Link to={here}>{current.title}</Link>
        <span className={styles.sep}>/</span>
        <Link to={`${here}/history`}>History</Link>
      </nav>
      <div className="page-header">
        <div>
          <h1 className={styles.heading}>
            {dateTime(revision.updated_at)} <RevisionBadges revision={revision} />
          </h1>
          <p>
            Saved by {revision.author_name}
            {revision.published_at &&
              ` · published by ${revision.published_by_name} ${dateTime(revision.published_at)}`}
            {note && ` · ${note}`}
          </p>
        </div>
        {!revision.is_current && (
          <button className="primary" disabled={action.busy} onClick={restore}>
            Restore this version
          </button>
        )}
      </div>
      {action.error && <p className="alert error">{action.error}</p>}

      <div className={styles.bar}>
        <div className={styles.tabs} role="tablist" aria-label="Show">
          {(
            [
              ['content', 'This version'],
              ['changes', 'Changes made'],
              ['current', revision.is_current ? null : 'Compared with now'],
            ] as [Mode, string | null][]
          ).map(
            ([value, label]) =>
              label && (
                <button
                  key={value}
                  role="tab"
                  aria-selected={mode === value}
                  className={mode === value ? styles.selected : undefined}
                  onClick={() => setMode(value)}
                >
                  {label}
                </button>
              ),
          )}
        </div>
        <span className="row">
          <Link
            className={`button small ${older ? '' : styles.off}`}
            to={older ? `${here}/history/${older.revision_id}` : '#'}
            aria-disabled={!older}
          >
            ← Older
          </Link>
          <Link
            className={`button small ${newer ? '' : styles.off}`}
            to={newer ? `${here}/history/${newer.revision_id}` : '#'}
            aria-disabled={!newer}
          >
            Newer →
          </Link>
        </span>
      </div>

      {mode === 'content' && (
        <article className="card">
          <h2 className={styles.title}>{revision.title}</h2>
          {detail.body_md.trim() ? (
            <MarkdownView markdown={detail.body_md} />
          ) : (
            <p className="muted">This version is empty.</p>
          )}
        </article>
      )}
      {mode === 'changes' && (
        <>
          <p className="muted">
            {detail.previous ? (
              <>
                <Removed>Removed</Removed> and <Added>added</Added> since the version before it, of{' '}
                {dateTime(detail.previous.updated_at)}.
              </>
            ) : (
              'The first version of the page.'
            )}
          </p>
          <Diff
            before={detail.previous ?? { title: '', body_md: '' }}
            after={shown}
            same="Nothing changed from the version before it."
          />
        </>
      )}
      {mode === 'current' && (
        <>
          <p className="muted">
            <Removed>Removed</Removed> lines are in this version; <Added>added</Added> lines are in
            {page.draft ? ' the draft' : ' the page'} now.
          </p>
          <Diff before={shown} after={current} same="This version matches the page as it is now." />
        </>
      )}
    </div>
  )
}
