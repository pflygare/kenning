import type { Editor } from '@tiptap/react'
import { type KeyboardEvent, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { apiGet } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import type { QuickPage } from '../api/types/QuickPage'
import type { QuickResults } from '../api/types/QuickResults'
import { headingsFromMarkdown } from './headings'
import { loadPage, pageLinkHref, shownVersion } from './pageLinks'
import styles from './PageLinkPicker.module.css'

type Point = { x: number; y: number }
type Range = { from: number; to: number }
type SectionRow = { kind: 'section'; id: string | null; text: string; level: number }
type Row = { kind: 'page'; page: QuickPage } | SectionRow

/** Words of `text` that all appear in `query`'s words, ignoring case. */
const matches = (text: string, query: string) =>
  query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => text.toLowerCase().includes(word))

/**
 * Finds a page by title and links to it, or to one of its sections. Over selected text the
 * text becomes the link; otherwise the page's title is added as one.
 */
export default function PageLinkPicker({
  editor,
  org,
  onClose,
}: {
  editor: Editor
  org: string
  onClose: () => void
}) {
  const [range] = useState<Range>(() => {
    const { from, to } = editor.state.selection
    return { from, to }
  })
  const [at, setAt] = useState<Point>(() => {
    const coords = editor.view.coordsAtPos(range.from)
    return { x: coords.left, y: coords.bottom + 6 }
  })
  const [q, setQ] = useState('')
  const [pages, setPages] = useState<QuickPage[] | null>(null)
  // The page whose sections are listed, once one is opened.
  const [page, setPage] = useState<{ ref: QuickPage; detail: PageDetail | null } | null>(null)
  const [index, setIndex] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const panel = useRef<HTMLDivElement>(null)
  const input = useRef<HTMLInputElement>(null)
  const latest = useRef('')

  useEffect(() => {
    if (page) return
    latest.current = q
    const timer = window.setTimeout(() => {
      apiGet<QuickResults>(`/orgs/${org}/search/quick?q=${encodeURIComponent(q)}`)
        .then((r) => {
          if (latest.current !== q) return
          setPages(r.pages)
          setIndex(0)
        })
        .catch(() => setPages([]))
    }, 80)
    return () => window.clearTimeout(timer)
  }, [org, q, page])

  // Close on a click elsewhere.
  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (!(event.target instanceof Node) || !panel.current?.contains(event.target)) onClose()
    }
    window.addEventListener('mousedown', close)
    return () => window.removeEventListener('mousedown', close)
  }, [onClose])

  // Keep the panel inside the window once its size is known.
  useLayoutEffect(() => {
    if (!panel.current) return
    const { width, height } = panel.current.getBoundingClientRect()
    const x = Math.max(8, Math.min(at.x, window.innerWidth - width - 8))
    const above = at.y + height > window.innerHeight - 8
    const y = above ? Math.max(8, at.y - height - 32) : at.y
    if (x !== at.x || y !== at.y) setAt({ x, y })
  }, [at])

  const sections: SectionRow[] = page?.detail
    ? [
        // While searching, Enter should pick the best section, not the page.
        ...(q.trim() ? [] : [{ kind: 'section', id: null, text: 'Whole page', level: 0 } as const]),
        ...headingsFromMarkdown(shownVersion(page.detail).body_md)
          .filter((h) => matches(h.text, q))
          .map((h): SectionRow => ({ kind: 'section', id: h.id, text: h.text, level: h.level })),
      ]
    : []
  const rows: Row[] = page ? sections : (pages ?? []).map((p) => ({ kind: 'page', page: p }))
  const top = Math.min(...sections.flatMap((r) => (r.id ? [r.level] : [])))

  const insert = (target: QuickPage, section: { id: string; text: string } | null) => {
    const href = pageLinkHref(org, target, section?.id)
    const chain = editor.chain().focus()
    if (range.from === range.to) {
      const text = section ? `${target.title} › ${section.text}` : target.title
      chain
        .insertContentAt(range, { type: 'text', text, marks: [{ type: 'link', attrs: { href } }] })
        .command(({ tr }) => {
          tr.setStoredMarks([])
          return true
        })
        .run()
    } else {
      chain.setTextSelection(range).setLink({ href }).run()
    }
    onClose()
  }

  const openSections = (target: QuickPage) => {
    setPage({ ref: target, detail: null })
    setQ('')
    setIndex(0)
    setError(null)
    loadPage(org, target.short_id)
      .then((detail) => setPage((p) => (p?.ref === target ? { ref: target, detail } : p)))
      .catch(() => setError("Couldn't load this page's sections."))
    input.current?.focus()
  }

  const back = () => {
    setPage(null)
    setQ('')
    setIndex(0)
    input.current?.focus()
  }

  const choose = (row: Row) => {
    if (row.kind === 'page') insert(row.page, null)
    else if (page) insert(page.ref, row.id ? { id: row.id, text: row.text } : null)
  }

  const onKeyDown = (event: KeyboardEvent) => {
    const row = rows[Math.min(index, rows.length - 1)]
    if (event.key === 'ArrowDown' && rows.length) setIndex((index + 1) % rows.length)
    else if (event.key === 'ArrowUp' && rows.length)
      setIndex((index - 1 + rows.length) % rows.length)
    else if (event.key === 'Enter' && row) choose(row)
    else if (event.key === 'ArrowRight' && row?.kind === 'page' && !q.length) openSections(row.page)
    else if (event.key === 'Tab' && row?.kind === 'page') openSections(row.page)
    else if (event.key === 'Backspace' && page && !q) back()
    else if (event.key === 'Escape') {
      onClose()
      editor.commands.focus()
    } else return
    event.preventDefault()
  }

  return (
    <div
      ref={panel}
      className={styles.picker}
      style={{ left: at.x, top: at.y }}
      role="dialog"
      aria-label="Link to page"
    >
      {page && (
        <button type="button" className={styles.back} onClick={back}>
          ‹ <span className={styles.backTitle}>{page.ref.title}</span>
        </button>
      )}
      <input
        ref={input}
        autoFocus
        className={styles.search}
        placeholder={page ? 'Find a section' : 'Find a page to link to'}
        aria-label={page ? 'Find a section' : 'Find a page'}
        value={q}
        onChange={(e) => {
          setQ(e.target.value)
          setIndex(0)
        }}
        onKeyDown={onKeyDown}
      />
      <div className={styles.list} role="listbox" aria-label={page ? 'Sections' : 'Pages'}>
        {rows.map((row, i) =>
          row.kind === 'page' ? (
            <div
              key={row.page.short_id}
              role="option"
              aria-selected={i === index}
              className={`${styles.row} ${i === index ? styles.active : ''}`}
              onMouseEnter={() => setIndex(i)}
            >
              <button type="button" className={styles.choose} onClick={() => choose(row)}>
                <span className={styles.title}>{row.page.title}</span>
                {!row.page.published && <span className="badge draft">Draft</span>}
              </button>
              <button
                type="button"
                className={styles.sections}
                title="Link to a section of this page"
                aria-label={`Sections of ${row.page.title}`}
                onClick={() => openSections(row.page)}
              >
                § ›
              </button>
            </div>
          ) : (
            <button
              key={row.id ?? ''}
              type="button"
              role="option"
              aria-selected={i === index}
              className={`${styles.row} ${styles.choose} ${i === index ? styles.active : ''}`}
              style={row.id ? { paddingLeft: `${0.6 + (row.level - top) * 0.9}rem` } : undefined}
              onMouseEnter={() => setIndex(i)}
              onClick={() => choose(row)}
            >
              {row.id ? (
                <span className={styles.title}>{row.text}</span>
              ) : (
                <span className={styles.whole}>{row.text}</span>
              )}
            </button>
          ),
        )}
        {!page && pages?.length === 0 && <p className={styles.empty}>No pages match.</p>}
        {page && !page.detail && !error && <p className={styles.empty}>Loading…</p>}
        {page?.detail && sections.every((r) => !r.id) && (
          <p className={styles.empty}>{q ? 'No sections match.' : 'This page has no sections.'}</p>
        )}
        {error && <p className={styles.empty}>{error}</p>}
      </div>
      <p className={styles.hint}>
        {page
          ? 'Enter links the section · Backspace goes back'
          : 'Enter links the page · Tab picks a section'}
      </p>
    </div>
  )
}
