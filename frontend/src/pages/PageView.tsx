import { useEffect, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { apiDelete, apiPost } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import type { PublishRequest } from '../api/types/PublishRequest'
import { useAction } from '../hooks/useAction'
import { pagePath, timeAgo } from './format'
import MarkdownView from './MarkdownView'
import type { Heading } from './headings'
import Outline from './Outline'
import PageMeta from './PageMeta'
import styles from './PageView.module.css'
import { usePage } from './usePage'

/** A page as readers see it, with the draft controls for editors. */
export default function PageView() {
  const { org, api, page, setPage, error } = usePage()
  const navigate = useNavigate()
  const location = useLocation()
  const action = useAction()
  const [headings, setHeadings] = useState<Heading[]>([])

  // Keep the address in step with the title (slugs change on publish).
  useEffect(() => {
    if (page && location.pathname !== pagePath(org.slug, page)) {
      navigate(pagePath(org.slug, page) + location.hash, { replace: true })
    }
  }, [page, org.slug, location.pathname, location.hash, navigate])

  if (error) {
    return <p className="alert error">{error}</p>
  }
  if (!page) {
    return <p className="status">Loading…</p>
  }

  const shown = page.published ?? page.draft!
  const here = pagePath(org.slug, page)

  const publish = () =>
    action.run(async () => {
      const body: PublishRequest = { revision_id: page.draft!.revision_id }
      setPage(await apiPost<PageDetail>(`${api}/publish`, body))
    })
  const discard = () => {
    if (
      !window.confirm('Discard the unpublished changes? Readers keep seeing the published version.')
    )
      return
    void action.run(async () => setPage(await apiDelete<PageDetail>(`${api}/draft`)))
  }
  const archive = () => {
    if (!window.confirm(`Archive "${shown.title}"? It disappears from the page list.`)) return
    void action.run(async () => {
      await apiPost(`${api}/archive`)
      navigate(`/${org.slug}`, { replace: true })
    })
  }

  return (
    <>
      <article className={styles.page}>
        {page.draft && (
          <div className={`notice ${styles.banner}`}>
            <span>
              {page.published ? (
                <>
                  <strong>Unpublished changes</strong> by {page.draft.author_name},{' '}
                  {timeAgo(page.draft.updated_at)}. Readers still see the published version.
                </>
              ) : (
                <>
                  <strong>Draft.</strong> This page hasn't been published yet.
                </>
              )}
            </span>
            <span className="row">
              {page.published && (
                <Link className="button small" to={`${here}/compare`}>
                  Compare
                </Link>
              )}
              <Link className="button small" to={`${here}/edit`}>
                Edit draft
              </Link>
              <button
                className="small primary"
                disabled={action.busy}
                onClick={() => void publish()}
              >
                Publish
              </button>
            </span>
          </div>
        )}
        {action.error && <p className="alert error">{action.error}</p>}

        <header className={styles.header}>
          <h1>{shown.title}</h1>
          <div className="row">
            {!page.draft && (
              <Link className="button" to={`${here}/edit`}>
                Edit
              </Link>
            )}
            <details className={styles.more}>
              <summary className="button" aria-label="More actions">
                ⋯
              </summary>
              <div className={styles.menu}>
                <Link className={`button ${styles.menuLink}`} to={`${here}/history`}>
                  Page history
                </Link>
                {page.draft && page.published && (
                  <button onClick={discard} disabled={action.busy}>
                    Discard unpublished changes
                  </button>
                )}
                <button className="danger" onClick={archive} disabled={action.busy}>
                  Archive page
                </button>
              </div>
            </details>
          </div>
        </header>
        <p className="muted">
          {page.published
            ? `Published by ${page.published_by_name} · ${timeAgo(page.published_at!)}`
            : `Started by ${shown.author_name} · ${timeAgo(page.created_at)}`}
          {' · '}
          <Link className={styles.historyLink} to={`${here}/history`}>
            History
          </Link>
        </p>
        <PageMeta org={org.slug} api={api} page={page} onChange={setPage} />

        {shown.body_md.trim() ? (
          <MarkdownView markdown={shown.body_md} onHeadings={setHeadings} />
        ) : (
          <p className="muted">This page is empty.</p>
        )}
      </article>
      {shown.body_md.trim() && <Outline headings={headings} />}
    </>
  )
}
