import { useEffect, useRef, useState } from 'react'
import { Link, useLocation, useNavigate, useSearchParams } from 'react-router'
import { apiGet, errorMessage } from '../api/client'
import type { Category } from '../api/types/Category'
import type { SearchResults } from '../api/types/SearchResults'
import type { Tag } from '../api/types/Tag'
import { valueListPath } from '../categories/paths'
import { pagePath, timeAgo } from '../pages/format'
import { useOrg } from '../routes/useOrg'
import TagChip from '../tags/TagChip'
import { useTopics } from '../topics/context'
import { flatten, topicPath } from '../topics/tree'
import Marked from './Marked'
import styles from './SearchPage.module.css'

const FILTERS = ['topic', 'tag', 'value'] as const

/** Full-text search over pages, with topic, tag and category filters kept in the address. */
export default function SearchPage() {
  // Searching from the ⌘K box starts over with what was typed there.
  const location = useLocation()
  const state = location.state as { searchId?: number } | null
  return <Search key={state?.searchId ?? 'here'} />
}

function Search() {
  const org = useOrg()
  const navigate = useNavigate()
  const location = useLocation()
  const [params] = useSearchParams()
  const { topics } = useTopics()
  const q = params.get('q') ?? ''
  const [text, setText] = useState(q)
  const [results, setResults] = useState<SearchResults | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [tags, setTags] = useState<Tag[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const latest = useRef('')

  useEffect(() => {
    const query = params.toString()
    latest.current = query
    if (!q.trim()) return
    apiGet<SearchResults>(`/orgs/${org.slug}/search?${query}`)
      .then((r) => {
        // Typing sends many searches; show only the answer for the latest.
        if (latest.current === query) {
          setResults(r)
          setError(null)
        }
      })
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [org.slug, params, q])

  useEffect(() => {
    apiGet<Tag[]>(`/orgs/${org.slug}/tags`)
      .then(setTags)
      .catch(() => setTags([]))
    apiGet<Category[]>(`/orgs/${org.slug}/categories`)
      .then(setCategories)
      .catch(() => setCategories([]))
  }, [org.slug])

  const update = (changes: Record<string, string>) => {
    const next = new URLSearchParams(params)
    for (const [key, value] of Object.entries(changes)) {
      if (value) next.set(key, value)
      else next.delete(key)
    }
    // Typing replaces the address as it goes, so Back leaves search instead of undoing letters.
    navigate(`/${org.slug}/search?${next}`, { replace: true, state: location.state })
  }

  // Search a moment after typing stops.
  useEffect(() => {
    if (text.trim() === q.trim()) return
    const timer = window.setTimeout(() => update({ q: text.trim() }), 200)
    return () => window.clearTimeout(timer)
  })

  const filtered = FILTERS.some((f) => params.get(f))
  const shown = q.trim() ? results : null

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Search</h1>
          <p>
            Results update as you type; every word matches the start of a word in a title, body or tag. Press{' '}
            <kbd>{navigator.platform.startsWith('Mac') ? '⌘' : 'Ctrl'}</kbd> <kbd>K</kbd> anywhere to find a
            page from any screen.
          </p>
        </div>
      </div>
      <form
        className={styles.bar}
        role="search"
        onSubmit={(e) => {
          e.preventDefault()
          update({ q: text.trim() })
        }}
      >
        <input
          autoFocus
          type="search"
          aria-label="Search pages"
          placeholder="Search pages"
          maxLength={200}
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
      </form>
      <div className={styles.filters}>
        <select
          aria-label="Topic"
          value={params.get('topic') ?? ''}
          onChange={(e) => update({ topic: e.target.value })}
        >
          <option value="">All topics</option>
          {flatten(topics ?? []).map(({ topic, depth }) => (
            <option key={topic.id} value={topic.id}>
              {'  '.repeat(depth)}
              {topic.name}
            </option>
          ))}
        </select>
        <select aria-label="Tag" value={params.get('tag') ?? ''} onChange={(e) => update({ tag: e.target.value })}>
          <option value="">Any tag</option>
          {tags.map((tag) => (
            <option key={tag.id} value={tag.slug}>
              {tag.name}
            </option>
          ))}
        </select>
        {categories.length > 0 && (
          <select
            aria-label="Category value"
            value={params.get('value') ?? ''}
            onChange={(e) => update({ value: e.target.value })}
          >
            <option value="">Any category</option>
            {categories.map((category) => (
              <optgroup key={category.id} label={category.name}>
                {category.values.map((value) => (
                  <option key={value.id} value={value.id}>
                    {value.name}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        )}
        {filtered && (
          <button type="button" className="small ghost" onClick={() => update({ topic: '', tag: '', value: '' })}>
            Clear filters
          </button>
        )}
      </div>

      {error && <p className="alert error">{error}</p>}
      {shown && shown.topics.length > 0 && (
        <div className={`chips ${styles.topics}`} aria-label="Matching topics">
          <span className="muted">Topics</span>
          {shown.topics.map((topic) => (
            <Link key={topic.id} to={topicPath(org.slug, topic)} className={styles.topic}>
              {topic.name}
            </Link>
          ))}
        </div>
      )}
      {shown && (
        <p className={`muted ${styles.count}`} aria-live="polite">
          {shown.pages.length === 0
            ? `No pages match “${q}”${filtered ? ' with these filters' : ''}.`
            : shown.pages.length === 1
              ? '1 page'
              : `${shown.pages.length} pages${shown.pages.length === 50 ? ' (the best 50)' : ''}`}
        </p>
      )}
      {shown && shown.pages.length > 0 && (
        <ol className={`card ${styles.results}`}>
          {shown.pages.map((hit) => (
            <li key={hit.short_id} className={styles.hit}>
              <div className={styles.head}>
                <Link to={pagePath(org.slug, hit)} className={styles.title}>
                  <Marked text={hit.title} />
                </Link>
                {!hit.published && <span className="badge draft">Draft</span>}
              </div>
              {hit.snippet.trim() && (
                <p className={styles.snippet}>
                  <Marked text={hit.snippet} />
                </p>
              )}
              <div className={`chips ${styles.meta}`}>
                {hit.topics.map((topic) => (
                  <Link key={topic.id} to={topicPath(org.slug, topic)} className={styles.topic}>
                    {topic.name}
                  </Link>
                ))}
                {hit.values.map((value) => (
                  <Link
                    key={value.id}
                    to={valueListPath(org.slug, value)}
                    className={`tag value ${value.color}`}
                    title={`${value.category}: ${value.name}`}
                  >
                    {value.name}
                  </Link>
                ))}
                {hit.tags.map((tag) => (
                  <TagChip key={tag.id} org={org.slug} tag={tag} />
                ))}
                <span className="muted">
                  {hit.updated_by_name} · {timeAgo(hit.updated_at)}
                </span>
              </div>
            </li>
          ))}
        </ol>
      )}
    </>
  )
}
