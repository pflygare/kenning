import { type MouseEvent, type ReactNode, useEffect, useRef, useState } from 'react'
import { copyLink } from './format'
import type { Heading } from './headings'
import styles from './SectionLinks.module.css'

/**
 * Wraps page content. Hovering a heading shows a button beside it that copies a link to
 * that section, ready to paste into another page.
 */
export default function SectionLinks({
  headings,
  linkBase,
  children,
}: {
  headings: Heading[]
  /** The page's address, without a section. */
  linkBase: string
  children: ReactNode
}) {
  const box = useRef<HTMLDivElement>(null)
  const [hovered, setHovered] = useState<{ heading: Heading; top: number } | null>(null)
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(() => {
    if (!copied) return
    const timer = window.setTimeout(() => setCopied(null), 1500)
    return () => window.clearTimeout(timer)
  }, [copied])

  const onMouseOver = (event: MouseEvent) => {
    if (!(event.target instanceof Element) || !box.current) return
    if (event.target.closest(`.${styles.button}`)) return
    const element = event.target.closest('h1, h2, h3')
    const heading = headings.find((h) => h.element === element)
    if (!heading) {
      setHovered(null)
      return
    }
    if (heading === hovered?.heading) return
    // Centered on the heading's first line, inside this wrapper.
    const outer = box.current.getBoundingClientRect()
    const rect = heading.element.getBoundingClientRect()
    const line = parseFloat(getComputedStyle(heading.element).lineHeight) || rect.height
    setHovered({ heading, top: rect.top - outer.top + Math.min(line, rect.height) / 2 })
  }

  const shown = hovered && headings.includes(hovered.heading) ? hovered : null
  return (
    <div
      ref={box}
      className={styles.box}
      onMouseOver={onMouseOver}
      onMouseLeave={() => setHovered(null)}
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
          onClick={() => {
            void copyLink(`${linkBase}#${encodeURIComponent(shown.heading.id)}`)
            setCopied(shown.heading.id)
          }}
        >
          {copied === shown.heading.id ? '✓' : '#'}
        </button>
      )}
    </div>
  )
}
