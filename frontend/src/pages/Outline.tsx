import { useEffect, useRef, useState } from 'react'
import type { Heading } from './headings'
import styles from './Outline.module.css'

/** Where headings stop counting as "passed" while scrolling: below the sticky app bar. */
const READING_LINE = 96

/**
 * The page's headings in a panel docked to the window's right edge. Follows along as you
 * scroll and jumps on click; shows nothing when there are no headings.
 */
export default function Outline({ headings }: { headings: Heading[] }) {
  const [current, setCurrent] = useState<string | null>(null)

  useEffect(() => {
    let frame = 0
    const update = () => {
      frame = 0
      const passed = headings.filter((h) => h.element.getBoundingClientRect().top <= READING_LINE)
      const atBottom =
        window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 2
      setCurrent((atBottom ? headings.at(-1) : passed.at(-1))?.id ?? headings[0]?.id ?? null)
    }
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    onScroll()
    window.addEventListener('scroll', onScroll, { passive: true })
    window.addEventListener('resize', onScroll)
    return () => {
      cancelAnimationFrame(frame)
      window.removeEventListener('scroll', onScroll)
      window.removeEventListener('resize', onScroll)
    }
  }, [headings])

  // Opening a link to a heading lands on it once, when the content has rendered.
  const landed = useRef(false)
  useEffect(() => {
    if (landed.current || !headings.length) return
    landed.current = true
    const id = decodeURIComponent(window.location.hash.slice(1))
    headings.find((h) => h.id === id)?.element.scrollIntoView()
  }, [headings])

  if (!headings.length) return null

  const top = Math.min(...headings.map((h) => h.level))
  return (
    <aside className={`${styles.dock} outline-dock`}>
      <nav className={styles.outline} aria-label="Outline">
        <p className={styles.title}>Outline</p>
        <ul>
          {headings.map((h) => (
            <li key={h.id} style={{ paddingLeft: `${(h.level - top) * 0.875}rem` }}>
              <a
                href={`#${h.id}`}
                className={h.id === current ? styles.current : undefined}
                aria-current={h.id === current ? 'location' : undefined}
                onClick={(e) => {
                  e.preventDefault()
                  h.element.scrollIntoView({ behavior: 'smooth' })
                  history.replaceState(history.state, '', `#${h.id}`)
                  setCurrent(h.id)
                }}
              >
                {h.text}
              </a>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  )
}
