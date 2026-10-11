import { type MouseEvent, type ReactNode, useEffect, useRef, useState } from 'react'
import menuStyles from './EditorMenus.module.css'
import { copyLink } from './format'
import type { Heading } from './headings'
import styles from './SectionLinks.module.css'

/** How long the button stays after the pointer leaves the heading, to reach it. */
const HIDE_DELAY_MS = 400

/**
 * Wraps page content. Hovering a heading shows a 🔗 button beside it that copies a link
 * to that section, ready to paste into another page. With `menu`, right-clicking a
 * heading offers the same (Shift + right-click opens the browser's own menu).
 */
export default function SectionLinks({
  headings,
  linkBase,
  menu = false,
  children,
}: {
  headings: Heading[]
  /** The page's address, without a section. */
  linkBase: string
  menu?: boolean
  children: ReactNode
}) {
  const box = useRef<HTMLDivElement>(null)
  const panel = useRef<HTMLDivElement>(null)
  const hideTimer = useRef<number | undefined>(undefined)
  const [hovered, setHovered] = useState<{ heading: Heading; top: number } | null>(null)
  const [copied, setCopied] = useState<string | null>(null)
  const [opened, setOpened] = useState<{ heading: Heading; x: number; y: number } | null>(null)

  useEffect(() => {
    if (!copied) return
    const timer = window.setTimeout(() => setCopied(null), 1500)
    return () => window.clearTimeout(timer)
  }, [copied])

  useEffect(() => () => window.clearTimeout(hideTimer.current), [])

  // The right-click menu closes on a click elsewhere, scrolling or Escape.
  useEffect(() => {
    if (!opened) return
    const start = window.scrollY
    const close = (event: Event) => {
      if (event.target instanceof Node && panel.current?.contains(event.target)) return
      if (event.type === 'scroll' && window.scrollY === start) return
      if (event instanceof KeyboardEvent && event.key !== 'Escape') return
      setOpened(null)
    }
    window.addEventListener('mousedown', close)
    window.addEventListener('scroll', close, true)
    window.addEventListener('keydown', close)
    return () => {
      window.removeEventListener('mousedown', close)
      window.removeEventListener('scroll', close, true)
      window.removeEventListener('keydown', close)
    }
  }, [opened])

  const headingAt = (target: EventTarget) => {
    const element = target instanceof Element ? target.closest('h1, h2, h3') : null
    return headings.find((h) => h.element === element)
  }

  const show = (heading: Heading) => {
    window.clearTimeout(hideTimer.current)
    if (heading === hovered?.heading || !box.current) return
    // Centered on the heading's first line, inside this wrapper.
    const outer = box.current.getBoundingClientRect()
    const rect = heading.element.getBoundingClientRect()
    const line = parseFloat(getComputedStyle(heading.element).lineHeight) || rect.height
    setHovered({ heading, top: rect.top - outer.top + Math.min(line, rect.height) / 2 })
  }

  const hideSoon = () => {
    window.clearTimeout(hideTimer.current)
    hideTimer.current = window.setTimeout(() => setHovered(null), HIDE_DELAY_MS)
  }

  const onMouseOver = (event: MouseEvent) => {
    if (event.target instanceof Element && event.target.closest(`.${styles.button}`)) {
      window.clearTimeout(hideTimer.current)
      return
    }
    const heading = headingAt(event.target)
    if (heading) show(heading)
    else if (hovered) hideSoon()
  }

  const onContextMenu = (event: MouseEvent) => {
    if (!menu || event.shiftKey) return
    const heading = headingAt(event.target)
    if (!heading) return
    event.preventDefault()
    setOpened({ heading, x: event.clientX, y: event.clientY })
  }

  const copy = (heading: Heading) => {
    void copyLink(`${linkBase}#${encodeURIComponent(heading.id)}`)
    setCopied(heading.id)
  }

  const shown = hovered && headings.includes(hovered.heading) ? hovered : null
  return (
    <div
      ref={box}
      className={styles.box}
      onMouseOver={onMouseOver}
      onMouseLeave={hideSoon}
      onContextMenu={onContextMenu}
    >
      {children}
      {shown && (
        <button
          type="button"
          className={styles.button}
          style={{ top: shown.top }}
          title={copied === shown.heading.id ? 'Link copied' : 'Copy a link to this section'}
          aria-label={`Copy a link to ${shown.heading.text}`}
          onMouseDown={(e) => e.preventDefault()}
          onClick={() => copy(shown.heading)}
        >
          {copied === shown.heading.id ? '✓' : '🔗'}
        </button>
      )}
      {opened && (
        <div
          ref={panel}
          className={menuStyles.menu}
          role="menu"
          aria-label="Section"
          style={{
            left: Math.min(opened.x, window.innerWidth - 248),
            top: Math.min(opened.y, window.innerHeight - 96),
          }}
          onContextMenu={(e) => e.preventDefault()}
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              copy(opened.heading)
              setOpened(null)
            }}
          >
            <span className={menuStyles.icon}>🔗</span>
            Copy link to section
          </button>
          <p className={menuStyles.footnote}>Shift + right-click for the browser menu</p>
        </div>
      )}
    </div>
  )
}
