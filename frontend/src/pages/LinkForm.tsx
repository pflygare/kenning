import { type FormEvent, useEffect, useLayoutEffect, useRef, useState } from 'react'
import styles from './LinkEditor.module.css'

export type Target = {
  from: number
  to: number
  text: string
  href: string
  at: { x: number; y: number }
}

/** The link editor's form: the words a link shows and where it goes. */
export default function LinkForm({
  target,
  onSave,
  onRemove,
  onCancel,
  onFindPage,
}: {
  target: Target
  onSave: (text: string, href: string) => void
  onRemove: (() => void) | null
  onCancel: () => void
  onFindPage: (() => void) | null
}) {
  const [text, setText] = useState(target.text)
  const [href, setHref] = useState(target.href)
  const [at, setAt] = useState(target.at)
  const panel = useRef<HTMLFormElement>(null)

  // Keep the form inside the window once its size is known.
  useLayoutEffect(() => {
    const box = panel.current?.getBoundingClientRect()
    if (!box) return
    const x = Math.max(8, Math.min(at.x, window.innerWidth - box.width - 8))
    const y = at.y + box.height > window.innerHeight - 8 ? at.y - box.height - 30 : at.y
    if (x !== at.x || y !== at.y) setAt({ x, y })
  }, [at])

  useEffect(() => {
    const away = (event: MouseEvent) => {
      if (!(event.target instanceof Node) || !panel.current?.contains(event.target)) onCancel()
    }
    window.addEventListener('mousedown', away)
    return () => window.removeEventListener('mousedown', away)
  }, [onCancel])

  const submit = (event: FormEvent) => {
    event.preventDefault()
    onSave(text, href)
  }

  return (
    <form
      ref={panel}
      className={styles.panel}
      style={{ left: at.x, top: at.y }}
      aria-label={target.href ? 'Edit link' : 'Add link'}
      onSubmit={submit}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          onCancel()
        }
      }}
    >
      <label>
        Text
        <input
          value={text}
          placeholder="Words the link shows"
          onChange={(e) => setText(e.target.value)}
          autoFocus={!!target.href || !target.text}
        />
      </label>
      <label>
        Link to
        <input
          value={href}
          placeholder="https://… or a Kenning page address"
          onChange={(e) => setHref(e.target.value)}
          autoFocus={!target.href && !!target.text}
        />
      </label>
      <div className={styles.actions}>
        {onFindPage && (
          <button type="button" className={styles.find} onClick={onFindPage}>
            ↗ Find a page…
          </button>
        )}
        {onRemove && (
          <button type="button" onClick={onRemove}>
            Remove link
          </button>
        )}
        <button type="submit" className="primary">
          Save
        </button>
      </div>
    </form>
  )
}
