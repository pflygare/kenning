import { NodeViewContent, NodeViewWrapper, type ReactNodeViewProps } from '@tiptap/react'
import { useEffect, useRef, useState } from 'react'
import { tones, type Tone } from './callout'
import styles from './CalloutView.module.css'

/**
 * A note's frame: its label on top, then its content. While editing, the label is a button
 * that picks the note's style or removes the note.
 */
export default function CalloutView({
  node,
  editor,
  updateAttributes,
  getPos,
}: ReactNodeViewProps) {
  const tone = tones.find((t) => t.id === node.attrs.tone) ?? tones[0]
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const close = (event: Event) => {
      if (!(event.target instanceof Node) || !box.current?.contains(event.target)) setOpen(false)
    }
    const escape = (event: KeyboardEvent) => event.key === 'Escape' && setOpen(false)
    window.addEventListener('mousedown', close)
    window.addEventListener('keydown', escape)
    return () => {
      window.removeEventListener('mousedown', close)
      window.removeEventListener('keydown', escape)
    }
  }, [open])

  const choose = (id: Tone) => {
    setOpen(false)
    updateAttributes({ tone: id })
  }

  const remove = () => {
    setOpen(false)
    const pos = getPos()
    if (pos === undefined) return
    editor
      .chain()
      .focus()
      .setTextSelection(pos + 2)
      .unsetCallout()
      .run()
  }

  return (
    <NodeViewWrapper as="aside" data-callout={tone.id} className={styles.callout}>
      <div className={styles.label} contentEditable={false} ref={box}>
        {editor.isEditable ? (
          <button
            type="button"
            className={styles.pick}
            aria-haspopup="menu"
            aria-expanded={open}
            title="Change the note's style"
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => setOpen((o) => !o)}
          >
            <span aria-hidden="true">{tone.icon}</span> {tone.label}
            <span className={styles.caret} aria-hidden="true">
              ▾
            </span>
          </button>
        ) : (
          <span>
            <span aria-hidden="true">{tone.icon}</span> {tone.label}
          </span>
        )}
        {open && (
          <div className={styles.menu} role="menu" aria-label="Note style">
            {tones.map((t) => (
              <button
                key={t.id}
                type="button"
                role="menuitemradio"
                aria-checked={t.id === tone.id}
                data-callout={t.id}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => choose(t.id)}
              >
                <span className={styles.swatch} aria-hidden="true">
                  {t.icon}
                </span>
                {t.label}
              </button>
            ))}
            <div className={styles.separator} />
            <button
              type="button"
              role="menuitem"
              onMouseDown={(e) => e.preventDefault()}
              onClick={remove}
            >
              <span className={styles.swatch} aria-hidden="true" />
              Remove note
            </button>
          </div>
        )}
      </div>
      <NodeViewContent className={styles.body} />
    </NodeViewWrapper>
  )
}
