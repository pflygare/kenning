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
import styles from './SearchBox.module.css'

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

const isMac = typeof navigator !== 'undefined' && navigator.platform.startsWith('Mac')

/**
 * The search field in the top bar. While focused it lists pages and topics by
 * title and pages by words in their text; Ctrl+K (⌘K) focuses it from anywhere.
 */
export default function SearchBox({ org }: { org: string }) {
  const navigate = useNavigate()
  const [q, setQ] = useState('')
  const [results, setResults] = useState<QuickResults | null>(null)
  const [text, setText] = useState<{ q: string; hits: SearchHit[] }>({ q: '', hits: [] })
  // -1 while nothing is picked: Enter then searches all pages.
  const [index, setIndex] = useState(-1)
  const [open, setOpen] = useState(false)
  const latest = useRef('')
  const input = useRef<HTMLInputElement>(null)

  useEffect(() => {
    const focus = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        input.current?.focus()
        input.current?.select()
      }
    }
    document.addEventListener('keydown', focus)
    return () => document.removeEventListener('keydown', focus)
  }, [])

  const close = () => {
    setOpen(false)
    input.current?.blur()
  }

  useEffect(() => {
    latest.current = q
    if (!open) return
    const timer = window.setTimeout(() => {
      apiGet<QuickResults>(`/orgs/${org}/search/quick?q=${encodeURIComponent(q)}`)
        .then((r) => {
          // Answers can arrive out of order; keep only the one for what is typed now.
          if (latest.current === q) {
            setResults(r)
            setIndex(-1)
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
  }, [org, q, open])

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
    close()
    setQ('')
    navigate(option.to, option.kind === 'search' ? { state: { searchId: Date.now() } } : undefined)
  }

  return (
    <div className={styles.box} role="search">
      <span className={styles.icon} aria-hidden="true">
        <SearchIcon />
      </span>
      <input
        ref={input}
        className={styles.input}
        type="text"
        aria-label="Search"
        aria-expanded={open}
        aria-controls="search-matches"
        role="combobox"
        placeholder="Search pages and topics"
        maxLength={200}
        value={q}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onChange={(e) => {
          setQ(e.target.value)
          setOpen(true)
        }}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown') setIndex((i) => Math.min(i + 1, options.length - 1))
          else if (e.key === 'ArrowUp') setIndex((i) => Math.max(i - 1, -1))
          else if (e.key === 'Enter') go(options[index] ?? options.find((o) => o.kind === 'search'))
          else if (e.key === 'Escape') close()
          else return
          e.preventDefault()
        }}
      />
      {!open && <kbd className={styles.shortcut}>{isMac ? '⌘K' : 'Ctrl K'}</kbd>}
      {open && results && (
        // Clicking a match must not blur the field before the click lands.
        <div className={styles.panel} onMouseDown={(e) => e.preventDefault()}>
          <ul className={styles.list} role="listbox" id="search-matches" aria-label="Matches">
            {!q.trim() && results.pages.length > 0 && <li className={styles.label}>Recently changed</li>}
            {options.map((option, i) => (
              <li
                key={option.key}
                role="option"
                aria-selected={i === index}
                className={`${styles.option} ${i === index ? styles.selected : ''}`}
                onMouseMove={() => setIndex(i)}
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
            {q.trim() && text.q === q && options.length === 1 && (
              <li className={styles.empty}>No pages or topics match.</li>
            )}
          </ul>
          <div className={styles.footer}>
            <span>
              <kbd>↑</kbd> <kbd>↓</kbd> to pick
            </span>
            <span>
              <kbd>Enter</kbd> to search, or open what you picked
            </span>
            <span>
              <kbd>Esc</kbd> to close
            </span>
          </div>
        </div>
      )}
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
