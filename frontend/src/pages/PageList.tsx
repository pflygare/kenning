import { useEffect, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { Category } from '../api/types/Category'
import type { CreatePageRequest } from '../api/types/CreatePageRequest'
import type { PageDetail } from '../api/types/PageDetail'
import type { PageSummary } from '../api/types/PageSummary'
import type { Tag } from '../api/types/Tag'
import { useAction } from '../hooks/useAction'
import { useOrg } from '../routes/useOrg'
import { pagePath } from './format'
import PageRows from './PageRows'
import styles from './PageList.module.css'

/** The organization's home: its pages, most recently changed first, optionally by tag. */
export default function PageList() {
  const [params] = useSearchParams()
  const tagSlug = params.get('tag')
  const valueId = params.get('value')
  return <Pages key={`${tagSlug}/${valueId}`} tagSlug={tagSlug} valueId={valueId} />
}

function Pages({ tagSlug, valueId }: { tagSlug: string | null; valueId: string | null }) {
  const org = useOrg()
  const navigate = useNavigate()
  const [pages, setPages] = useState<PageSummary[] | null>(null)
  const [tag, setTag] = useState<Tag | null>(null)
  const [value, setValue] = useState<{ category: string; name: string; color: string } | null>(null)
  const [error, setError] = useState<string | null>(null)
  const create = useAction()

  useEffect(() => {
    const filter = new URLSearchParams()
    if (tagSlug) filter.set('tag', tagSlug)
    if (valueId) filter.set('value', valueId)
    const query = filter.size ? `?${filter}` : ''
    apiGet<PageSummary[]>(`/orgs/${org.slug}/pages${query}`)
      .then(setPages)
      .catch((err: unknown) => setError(errorMessage(err)))
    if (tagSlug) {
      apiGet<Tag[]>(`/orgs/${org.slug}/tags`)
        .then((tags) => setTag(tags.find((t) => t.slug === tagSlug) ?? null))
        .catch(() => setTag(null))
    }
    if (valueId) {
      apiGet<Category[]>(`/orgs/${org.slug}/categories`)
        .then((categories) => {
          for (const category of categories) {
            const found = category.values.find((v) => v.id === valueId)
            if (found) setValue({ category: category.name, name: found.name, color: found.color })
          }
        })
        .catch(() => setValue(null))
    }
  }, [org.slug, tagSlug, valueId])

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
          <p>Recently updated in {org.name}.</p>
        </div>
        <button className="primary" disabled={create.busy} onClick={() => void newPage()}>
          + New page
        </button>
      </div>
      {valueId && (
        <div className={styles.filter}>
          <span className="muted">{value?.category ?? 'Category'}</span>
          <span className={`tag value ${value?.color ?? ''}`}>
            {value?.name ?? '…'}
            <Link to={`/${org.slug}`} aria-label="Show all pages" className={styles.clear}>
              ×
            </Link>
          </span>
        </div>
      )}
      {tagSlug && (
        <div className={styles.filter}>
          <span className="muted">Tagged</span>
          <span className={`tag ${tag?.color ?? ''}`}>
            {tag?.name ?? tagSlug}
            <Link to={`/${org.slug}`} aria-label="Show all pages" className={styles.clear}>
              ×
            </Link>
          </span>
        </div>
      )}
      {(error || create.error) && <p className="alert error">{error ?? create.error}</p>}
      {pages?.length === 0 && !tagSlug && !valueId && (
        <div className="card empty">
          <h2>No pages yet</h2>
          <p>Write the first one: a team handbook, an onboarding guide, or meeting notes.</p>
          <button className="primary" disabled={create.busy} onClick={() => void newPage()}>
            Write a page
          </button>
        </div>
      )}
      {pages?.length === 0 && tagSlug && <p className="muted">No pages have this tag.</p>}
      {pages?.length === 0 && valueId && <p className="muted">No pages have this value.</p>}
      {pages && pages.length > 0 && <PageRows org={org.slug} pages={pages} />}
    </>
  )
}
