import { type ReactNode, useLayoutEffect, useRef, useState } from 'react'
import styles from './ChipColumn.module.css'

/**
 * Chips on one line, as many as fit the column. The rest become "+N", and
 * hovering or focusing that shows the whole list.
 */
export default function ChipColumn({
  chips,
  label,
  className = '',
}: {
  chips: { key: string; node: ReactNode }[]
  label: string
  className?: string
}) {
  const column = useRef<HTMLDivElement>(null)
  const measure = useRef<HTMLDivElement>(null)
  const [shown, setShown] = useState(chips.length)

  useLayoutEffect(() => {
    const box = column.current
    const ruler = measure.current
    if (!box || !ruler) return
    const fit = () => {
      const widths = [...ruler.children].map((child) => child.getBoundingClientRect().width)
      const more = widths.pop() ?? 0
      const gap = parseFloat(getComputedStyle(ruler).columnGap) || 0
      const room = box.clientWidth
      let used = 0
      let count = 0
      for (const [i, width] of widths.entries()) {
        const next = used + (i > 0 ? gap : 0) + width
        const left = widths.length - i - 1
        if (next + (left > 0 ? gap + more : 0) > room) break
        used = next
        count++
      }
      setShown(count)
    }
    fit()
    const observer = new ResizeObserver(fit)
    observer.observe(box)
    return () => observer.disconnect()
  }, [chips])

  const hidden = chips.length - shown
  return (
    <div ref={column} className={`${styles.column} ${className}`}>
      {chips.slice(0, shown).map((chip) => (
        <span key={chip.key} className={styles.chip}>
          {chip.node}
        </span>
      ))}
      {hidden > 0 && (
        <span className={styles.overflow}>
          <button type="button" className={styles.more} aria-label={`${hidden} more ${label}`}>
            +{hidden}
          </button>
          <span className={styles.panel} role="tooltip">
            {chips.map((chip) => (
              <span key={chip.key} className={styles.chip}>
                {chip.node}
              </span>
            ))}
          </span>
        </span>
      )}
      {/* Last, so the visible chips come first for anything that looks them up. */}
      <div ref={measure} className={styles.measure} aria-hidden>
        {chips.map((chip) => (
          <span key={chip.key} className={styles.chip}>
            {chip.node}
          </span>
        ))}
        <span className={styles.more}>+{chips.length}</span>
      </div>
    </div>
  )
}
