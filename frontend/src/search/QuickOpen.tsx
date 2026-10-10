import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router'
import { apiGet } from '../api/client'
import type { QuickResults } from '../api/types/QuickResults'
import type { SearchHit } from '../api/types/SearchHit'
import type { SearchResults } from '../api/types/SearchResults'
import { pagePath } from '../pages/format'
import { topicPath } from '../topics/tree'
import Marked from './Marked'
import { searchPath } from './paths'
import styles from './QuickOpen.module.css'

type Option = {
  key: string
  label: string
  kind: 'page' | 'text' | 'topic' | 'search'
  to: string
  draft?: boolean
  snippet?: string
}

/** Full-text matches shown under the title matches. */
const TEXT_MATCHES = 5

/** The ⌘K box: jump to a page or topic by title or by words in its text, or search everything. */
export default function QuickOpen({ org, onClose }: { org: string; onClose: () => void }) {
  const navigate = useNavigate()
  const [q, setQ] = useState('')
  const [results, setResults] = useState<QuickResults | null>(null)
  const [text, setText] = useState<{ q: string; hits: SearchHit[] }>({ q: '', hits: [] })
  const [index, setIndex] = useState(0)
  const latest = useRef('')

  useEffect(() => {
    latest.current = q
    const timer = window.setTimeout(() => {
      apiGet<QuickResults>(`/orgs/${org}/search/quick?q=${encodeURIComponent(q)}`)
        .then((r) => {
          // Answers can arrive out of order; keep only the one for what is typed now.
          if (latest.current === q) {
            setResults(r)
            setIndex(0)
          }
        })
        .catch(() => setResults({ pages: [], topics: [] }))
      if (q.trim()) {
        apiGet<SearchResults>(`/orgs/${org}/search?q=${encodeURIComponent(q)}`)
          .then((r) => {
            if (latest.current === q) setText({ q, hits: r.pages })
          })
          .catch(() => setText({ q, hits: [] }))
      }
    }, 80)
    return () => window.clearTimeout(timer)
  }, [org, q])

  const titled = new Set((results?.pages ?? []).map((p) => p.short_id))
  const textHits = q.trim() && text.q === q ? text.hits.filter((h) => !titled.has(h.short_id)).slice(0, TEXT_MATCHES) : []
  const options: Option[] = [
    ...(results?.pages ?? []).map((p) => ({
      key: `p${p.short_id}`,
      label: p.title,
      kind: 'page' as const,
      to: pagePath(org, p),
      draft: !p.published,
    })),
    ...textHits.map((h) => ({
      key: `x${h.short_id}`,
      label: h.title,
      kind: 'text' as const,
      to: pagePath(org, h),
      draft: !h.published,
      snippet: h.snippet,
    })),
    ...(results?.topics ?? []).map((t) => ({
      key: `t${t.short_id}`,
      label: t.name,
      kind: 'topic' as const,
      to: topicPath(org, t),
    })),
    ...(q.trim()
      ? [{ key: 'search', label: `Search all pages for “${q.trim()}”`, kind: 'search' as const, to: searchPath(org, q.trim()) }]
      : []),
  ]

  const go = (option: Option | undefined) => {
    if (!option) return
    onClose()
    navigate(option.to, option.kind === 'search' ? { state: { searchId: Date.now() } } : undefined)
  }

  return (
    <div className={styles.backdrop} onMouseDown={onClose}>
      <div
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-label="Go to a page"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          autoFocus
          className={styles.input}
          aria-label="Find a page"
          placeholder="Find a page or topic…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') setIndex((i) => Math.min(i + 1, options.length - 1))
            else if (e.key === 'ArrowUp') setIndex((i) => Math.max(i - 1, 0))
            else if (e.key === 'Enter') go(options[index])
            else if (e.key === 'Escape') onClose()
            else return
            e.preventDefault()
          }}
        />
        <ul className={styles.list} role="listbox" aria-label="Matches">
          {!q.trim() && results && results.pages.length > 0 && <li className={styles.label}>Recently changed</li>}
          {options.map((option, i) => (
            <li
              key={option.key}
              role="option"
              aria-selected={i === index}
              className={`${styles.option} ${i === index ? styles.selected : ''}`}
              onMouseEnter={() => setIndex(i)}
              onClick={() => go(option)}
            >
              <span className={styles.icon} aria-hidden="true">
                {option.kind === 'page' || option.kind === 'text' ? (
                  <PageIcon />
                ) : option.kind === 'topic' ? (
                  <FolderIcon />
                ) : (
                  <SearchIcon />
                )}
              </span>
              {option.kind === 'text' ? (
                <span className={styles.text}>
                  <Marked text={option.label} />
                  {option.snippet?.trim() && (
                    <span className={styles.snippet}>
                      <Marked text={option.snippet} />
                    </span>
                  )}
                </span>
              ) : (
                <span className={styles.text}>{option.label}</span>
              )}
              {option.draft && <span className="badge draft">Draft</span>}
              {option.kind === 'topic' && <span className="muted">Topic</span>}
            </li>
          ))}
          {q.trim() && results && text.q === q && options.length === 1 && (
            <li className={styles.empty}>No pages or topics match.</li>
          )}
        </ul>
        <div className={styles.footer}>
          <span>
            <kbd>↑</kbd> <kbd>↓</kbd> to move
          </span>
          <span>
            <kbd>Enter</kbd> to open
          </span>
          <span>
            <kbd>Esc</kbd> to close
          </span>
        </div>
      </div>
    </div>
  )
}

export function SearchIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </svg>
  )
}

function PageIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 3h8l4 4v14H6z" />
      <path d="M14 3v4h4" />
    </svg>
  )
}

function FolderIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
    </svg>
  )
}
