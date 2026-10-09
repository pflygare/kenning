import { type ReactNode, useEffect, useRef, useState } from 'react'

/**
 * A button that opens a panel below it. Clicking outside or pressing Escape
 * closes it; `children` gets a `close` function for actions that should too.
 */
export default function Popover({
  label,
  ariaLabel,
  triggerClass = '',
  align = 'right',
  children,
}: {
  label: ReactNode
  ariaLabel?: string
  triggerClass?: string
  align?: 'left' | 'right'
  children: (close: () => void) => ReactNode
}) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const close = (event: MouseEvent | KeyboardEvent) => {
      if (
        event instanceof KeyboardEvent
          ? event.key === 'Escape'
          : !ref.current?.contains(event.target as Node)
      ) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', close)
    document.addEventListener('keydown', close)
    return () => {
      document.removeEventListener('mousedown', close)
      document.removeEventListener('keydown', close)
    }
  }, [open])

  return (
    <div className="popover" ref={ref}>
      <button
        type="button"
        className={triggerClass}
        aria-label={ariaLabel}
        aria-haspopup="true"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {label}
      </button>
      {open && <div className={`popover-panel ${align}`}>{children(() => setOpen(false))}</div>}
    </div>
  )
}
