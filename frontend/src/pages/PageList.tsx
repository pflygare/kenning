import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { CreatePageRequest } from '../api/types/CreatePageRequest'
import type { PageDetail } from '../api/types/PageDetail'
import type { PageSummary } from '../api/types/PageSummary'
import { useAction } from '../hooks/useAction'
import { useOrg } from '../routes/useOrg'
import { pagePath, timeAgo } from './format'
import styles from './PageList.module.css'

/** The organization's home: its pages, most recently changed first. */
export default function PageList() {
  const org = useOrg()
  const navigate = useNavigate()
  const [pages, setPages] = useState<PageSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const create = useAction()

  useEffect(() => {
    apiGet<PageSummary[]>(`/orgs/${org.slug}/pages`)
      .then(setPages)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [org.slug])

  const newPage = () =>
    create.run(async () => {
      const body: CreatePageRequest = { title: 'Untitled', body_md: '' }
      const page = await apiPost<PageDetail>(`/orgs/${org.slug}/pages`, body)
      navigate(`${pagePath(org.slug, page)}/edit`, { state: { isNew: true } })
    })

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Pages</h1>
          <p>Everything written in {org.name}.</p>
        </div>
        <button className="primary" disabled={create.busy} onClick={() => void newPage()}>
          + New page
        </button>
      </div>
      {(error || create.error) && <p className="alert error">{error ?? create.error}</p>}
      {pages?.length === 0 && (
        <div className="card empty">
          <h2>No pages yet</h2>
          <p>Write the first one: a team handbook, an onboarding guide, or meeting notes.</p>
          <button className="primary" disabled={create.busy} onClick={() => void newPage()}>
            Write a page
          </button>
        </div>
      )}
      {pages && pages.length > 0 && (
        <ul className={`card ${styles.list}`}>
          {pages.map((page) => (
            <li key={page.short_id}>
              <Link to={pagePath(org.slug, page)} className={styles.item}>
                <span className={styles.title}>{page.title}</span>
                {!page.published ? (
                  <span className="badge draft">Draft</span>
                ) : (
                  page.has_draft && <span className="badge draft">Unpublished changes</span>
                )}
                <span className={`muted ${styles.meta}`}>
                  {page.updated_by_name} · {timeAgo(page.updated_at)}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </>
  )
}
