import {
  type MouseEvent,
  type ReactNode,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react'
import { createPortal } from 'react-dom'
import { Link, useNavigate } from 'react-router'
import type { PageDetail } from '../api/types/PageDetail'
import { pagePath } from './format'
import type { Heading } from './headings'
import MarkdownView from './MarkdownView'
import { loadPage, type PageLink, parsePageLink, shownVersion } from './pageLinks'
import styles from './LinkPreviews.module.css'

/** How long the pointer rests on a link before its preview opens, and lingers after. */
const OPEN_DELAY_MS = 350
const CLOSE_DELAY_MS = 250

type Preview = { link: PageLink; anchor: DOMRect; page: PageDetail | null; failed: boolean }

/**
 * Wraps page content. Resting the pointer on a link to another page shows that page in a
 * card, scrolled to the linked section. With `follow`, clicking such a link opens it here
 * instead of reloading the app, and links to this page's own sections scroll to them.
 */
export default function LinkPreviews({
  org,
  follow = false,
  onSection,
  children,
}: {
  org: string
  follow?: boolean
  /** Scrolls to a section of the page being shown, for `#section` links. */
  onSection?: (id: string) => void
  children: ReactNode
}) {
  const navigate = useNavigate()
  const [preview, setPreview] = useState<Preview | null>(null)
  const openTimer = useRef<number | undefined>(undefined)
  const closeTimer = useRef<number | undefined>(undefined)
  const hovered = useRef<HTMLAnchorElement | null>(null)

  useEffect(
    () => () => {
      window.clearTimeout(openTimer.current)
      window.clearTimeout(closeTimer.current)
    },
    [],
  )

  const linkAt = (target: EventTarget) =>
    target instanceof Element ? target.closest<HTMLAnchorElement>('a[href]') : null

  const onMouseOver = (event: MouseEvent) => {
    const anchor = linkAt(event.target)
    if (!anchor || anchor === hovered.current) return
    hovered.current = anchor
    const link = parsePageLink(anchor.getAttribute('href')!, org)
    window.clearTimeout(openTimer.current)
    if (!link) return
    window.clearTimeout(closeTimer.current)
    openTimer.current = window.setTimeout(() => {
      if (hovered.current !== anchor) return
      const rect = anchor.getBoundingClientRect()
      setPreview({ link, anchor: rect, page: null, failed: false })
      loadPage(org, link.shortId)
        .then((page) => setPreview((p) => (p?.link === link ? { ...p, page } : p)))
        .catch(() => setPreview((p) => (p?.link === link ? { ...p, failed: true } : p)))
    }, OPEN_DELAY_MS)
  }

  const onMouseOut = (event: MouseEvent) => {
    const anchor = linkAt(event.target)
    if (!anchor || (event.relatedTarget instanceof Node && anchor.contains(event.relatedTarget)))
      return
    hovered.current = null
    window.clearTimeout(openTimer.current)
    scheduleClose()
  }

  const scheduleClose = () => {
    window.clearTimeout(closeTimer.current)
    closeTimer.current = window.setTimeout(() => setPreview(null), CLOSE_DELAY_MS)
  }

  const onClick = (event: MouseEvent) => {
    if (!follow || event.defaultPrevented || event.button !== 0) return
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return
    const anchor = linkAt(event.target)
    if (!anchor) return
    const href = anchor.getAttribute('href')!
    if (href.startsWith('#')) {
      event.preventDefault()
      onSection?.(decodeURIComponent(href.slice(1)))
      return
    }
    if (!parsePageLink(href, org)) return
    event.preventDefault()
    window.clearTimeout(openTimer.current)
    hovered.current = null
    setPreview(null)
    const url = new URL(href, window.location.origin)
    navigate(url.pathname + url.hash)
  }

  return (
    <div onMouseOver={onMouseOver} onMouseOut={onMouseOut} onClickCapture={onClick}>
      {children}
      {preview &&
        createPortal(
          <PreviewCard
            org={org}
            preview={preview}
            onEnter={() => window.clearTimeout(closeTimer.current)}
            onLeave={scheduleClose}
            onOpen={() => setPreview(null)}
          />,
          document.body,
        )}
    </div>
  )
}

function PreviewCard({
  org,
  preview,
  onEnter,
  onLeave,
  onOpen,
}: {
  org: string
  preview: Preview
  onEnter: () => void
  onLeave: () => void
  onOpen: () => void
}) {
  const card = useRef<HTMLDivElement>(null)
  const body = useRef<HTMLDivElement>(null)
  const [place, setPlace] = useState<{ left: number; top: number } | null>(null)
  // Undefined until the page has rendered and its headings are known.
  const [section, setSection] = useState<Heading | null | undefined>(undefined)
  const { page, link, anchor } = preview
  const shown = page && shownVersion(page)

  // Below the link, or above it when there's no room; inside the window either way.
  useLayoutEffect(() => {
    if (!card.current) return
    const { width, height } = card.current.getBoundingClientRect()
    const left = Math.max(8, Math.min(anchor.left, window.innerWidth - width - 8))
    const below = anchor.bottom + 6
    const top =
      below + height <= window.innerHeight - 8 || anchor.top - height - 6 < 8
        ? below
        : anchor.top - height - 6
    setPlace((p) => (p?.left === left && p.top === top ? p : { left, top }))
  }, [anchor, page])

  // Bring the linked section to the top of the card, once it has rendered.
  const onHeadings = (headings: Heading[]) => {
    if (!headings.length && shown?.body_md.match(/^#/m)) return
    const heading = link.section ? (headings.find((h) => h.id === link.section) ?? null) : null
    if (heading?.element === section?.element) return
    setSection(heading)
    if (heading && body.current) {
      body.current.scrollTop +=
        heading.element.getBoundingClientRect().top - body.current.getBoundingClientRect().top - 8
    }
  }

  useEffect(() => {
    if (!section) return
    section.element.classList.add(styles.target)
    return () => section.element.classList.remove(styles.target)
  }, [section])

  const to = page ? pagePath(org, page) + (link.section ? `#${link.section}` : '') : '#'

  return (
    <div
      ref={card}
      className={styles.card}
      role="tooltip"
      style={place ?? { left: anchor.left, top: anchor.bottom + 6, visibility: 'hidden' }}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
    >
      {preview.failed ? (
        <p className={styles.note}>This page doesn't exist, or it was archived.</p>
      ) : !shown ? (
        <p className={styles.note}>Loading…</p>
      ) : (
        <>
          <div className={styles.head}>
            <Link to={to} className={styles.title} onClick={onOpen}>
              {shown.title}
            </Link>
            {!page.published && <span className="badge draft">Draft</span>}
          </div>
          {link.section && section === null && (
            <p className={styles.note}>The linked section isn't on this page any more.</p>
          )}
          <div ref={body} className={styles.body}>
            {shown.body_md.trim() ? (
              <MarkdownView markdown={shown.body_md} onHeadings={onHeadings} />
            ) : (
              <p className={styles.note}>This page is empty.</p>
            )}
          </div>
        </>
      )}
    </div>
  )
}
