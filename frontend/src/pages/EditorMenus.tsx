import type { Editor } from '@tiptap/react'
import {
  type KeyboardEvent,
  type MouseEvent,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react'
import { editImageText, imageSelected } from './captionedImage'
import { copyLink } from './format'
import { readHeadings } from './headings'
import {
  blockFormats,
  editLink,
  type Format,
  insertFormats,
  matchFormats,
  tableActions,
  textFormats,
} from './formats'
import styles from './EditorMenus.module.css'

type Point = { x: number; y: number }

/** Keep a menu of about `size` inside the window. */
function clamp({ x, y }: Point, size: Point): Point {
  return {
    x: Math.max(8, Math.min(x, window.innerWidth - size.x - 8)),
    y: Math.max(8, Math.min(y, window.innerHeight - size.y - 8)),
  }
}

/** `at` is just below the "/", `top` the top of its line, for a menu that opens upward. */
type Slash = { from: number; to: number; query: string; at: Point; top: number }

/** "/" followed by letters at the start of a block or after a space. */
const SLASH = /(?:^|\s)\/([a-z0-9]*)$/i

function findSlash(editor: Editor): Slash | null {
  const { selection } = editor.state
  const { $from, empty } = selection
  if (!empty || !$from.parent.isTextblock || $from.parent.type.name === 'codeBlock') return null
  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, '￼')
  const match = SLASH.exec(before)
  if (!match) return null
  const from = $from.pos - match[1].length - 1
  const coords = editor.view.coordsAtPos(from)
  return {
    from,
    to: $from.pos,
    query: match[1],
    at: { x: coords.left, y: coords.bottom + 4 },
    top: coords.top - 4,
  }
}

/**
 * Typing "/" opens a list of block styles; more letters narrow it ("/h2"),
 * Enter or a click applies one, Escape closes it. Returns the key handler the
 * editor's wrapper must call first, so arrows and Enter steer the menu.
 */
export function useSlashMenu(editor: Editor | null) {
  const [slash, setSlash] = useState<Slash | null>(null)
  const [index, setIndex] = useState(0)
  // A "/" the person closed with Escape stays closed until they type elsewhere.
  const dismissed = useRef<number | null>(null)

  useEffect(() => {
    if (!editor) return
    const update = () => {
      const found = findSlash(editor)
      if (!found || found.from !== dismissed.current) dismissed.current = null
      setSlash(found && found.from !== dismissed.current ? found : null)
      setIndex(0)
    }
    const close = () => setSlash(null)
    editor.on('transaction', update)
    editor.on('blur', close)
    return () => {
      editor.off('transaction', update)
      editor.off('blur', close)
    }
  }, [editor])

  const items = slash ? matchFormats(slash.query) : []
  const open = slash !== null && items.length > 0
  const list = useRef<HTMLDivElement>(null)
  // Below the line when it fits, otherwise above it, so the menu is never off screen.
  const [above, setAbove] = useState(false)

  useLayoutEffect(() => {
    const height = list.current?.offsetHeight
    if (!slash || !height) return
    setAbove(slash.at.y + height > window.innerHeight - 8 && slash.top - height >= 8)
  }, [slash, items.length])

  useLayoutEffect(() => {
    list.current?.querySelector('[aria-selected=true]')?.scrollIntoView({ block: 'nearest' })
  }, [index, open])

  const choose = (format: Format) => {
    if (!editor || !slash) return
    editor.chain().focus().deleteRange({ from: slash.from, to: slash.to }).run()
    format.apply(editor)
  }

  const onKeyDown = (event: KeyboardEvent) => {
    if (!open) return
    const move = (by: number) => setIndex((i) => (i + by + items.length) % items.length)
    if (event.key === 'ArrowDown') move(1)
    else if (event.key === 'ArrowUp') move(-1)
    else if (event.key === 'Enter' || event.key === 'Tab')
      choose(items[Math.min(index, items.length - 1)])
    else if (event.key === 'Escape') {
      dismissed.current = slash.from
      setSlash(null)
    } else return
    event.preventDefault()
    event.stopPropagation()
  }

  const menu = open ? (
    <div
      className={styles.menu}
      role="listbox"
      aria-label="Block styles"
      ref={list}
      style={{
        left: clamp(slash.at, { x: 240, y: 0 }).x,
        ...(above ? { bottom: window.innerHeight - slash.top } : { top: slash.at.y }),
      }}
      onMouseDown={(e) => e.preventDefault()}
    >
      {items.map((format, i) => (
        <button
          key={format.id}
          type="button"
          role="option"
          aria-selected={i === index}
          className={i === index ? styles.selected : undefined}
          onMouseEnter={() => setIndex(i)}
          onClick={() => choose(format)}
        >
          <span className={styles.icon}>{format.icon}</span>
          {format.label}
          <span className={styles.hint}>/{format.id}</span>
        </button>
      ))}
    </div>
  ) : null

  return { menu, onKeyDown }
}

/**
 * Right-clicking the editor opens text and block styles, applied to whatever
 * is selected. Shift + right-click opens the browser's own menu instead.
 */
export function useContextMenu(editor: Editor | null, linkBase?: string) {
  const [at, setAt] = useState<Point | null>(null)
  const ref = useRef<HTMLDivElement>(null)
  // The selection before the right button went down, which the browser may collapse.
  const before = useRef<{ from: number; to: number } | null>(null)

  useEffect(() => {
    if (!at) return
    const close = (event: Event) => {
      if (!(event.target instanceof Node) || !ref.current?.contains(event.target)) setAt(null)
    }
    const escape = (event: globalThis.KeyboardEvent) => {
      if (event.key === 'Escape') {
        setAt(null)
        editor?.commands.focus()
      }
    }
    // Focusing the editor can fire a scroll event without moving anything; ignore those.
    const start = { x: window.scrollX, y: window.scrollY }
    const scrolled = (event: Event) => {
      if (event.target !== document || window.scrollX !== start.x || window.scrollY !== start.y)
        close(event)
    }
    window.addEventListener('mousedown', close)
    window.addEventListener('scroll', scrolled, true)
    window.addEventListener('resize', close)
    window.addEventListener('keydown', escape)
    return () => {
      window.removeEventListener('mousedown', close)
      window.removeEventListener('scroll', scrolled, true)
      window.removeEventListener('resize', close)
      window.removeEventListener('keydown', escape)
    }
  }, [at, editor])

  // Fit the menu inside the window once its size is known.
  useEffect(() => {
    if (!at || !ref.current) return
    const { width, height } = ref.current.getBoundingClientRect()
    const fitted = clamp(at, { x: width, y: height })
    if (fitted.x !== at.x || fitted.y !== at.y) setAt(fitted)
  }, [at])

  const onMouseDown = (event: MouseEvent) => {
    if (editor && event.button === 2) {
      const { from, to } = editor.state.selection
      before.current = { from, to }
    }
  }

  const onContextMenu = (event: MouseEvent) => {
    if (!editor || event.shiftKey) return
    event.preventDefault()
    // Right-clicking the selection keeps it; anywhere else moves the cursor there.
    const { from, to } = before.current ?? editor.state.selection
    before.current = null
    const found = editor.view.posAtCoords({ left: event.clientX, top: event.clientY })
    const pos = found?.pos
    // Right-clicking an image selects it, for the image menu.
    if (found && found.inside >= 0 && editor.state.doc.nodeAt(found.inside)?.type.name === 'image')
      editor.commands.setNodeSelection(found.inside)
    else if (pos !== undefined && (pos < from || pos > to)) editor.commands.setTextSelection(pos)
    else editor.commands.setTextSelection({ from, to })
    editor.commands.focus()
    setAt({ x: event.clientX, y: event.clientY })
  }

  const run = (action: () => void) => {
    setAt(null)
    action()
  }

  const clipboard = async (action: 'cut' | 'copy' | 'paste') => {
    if (!editor) return
    editor.commands.focus()
    if (action !== 'paste') {
      document.execCommand(action)
      return
    }
    try {
      editor.view.pasteText(await navigator.clipboard.readText())
    } catch {
      window.alert('Your browser blocked pasting from this menu. Press Ctrl+V to paste.')
    }
  }

  const hasSelection = editor ? !editor.state.selection.empty : false

  // The heading the cursor is in, for "Copy link to section".
  const section = (() => {
    if (!at || !editor || !linkBase) return null
    const { node } = editor.view.domAtPos(editor.state.selection.from)
    const element = (node instanceof Element ? node : node.parentElement)?.closest('h1, h2, h3')
    return element
      ? (readHeadings(editor.view.dom).find((h) => h.element === element) ?? null)
      : null
  })()

  const imageMenu =
    at && editor && imageSelected(editor) ? (
      <div
        ref={ref}
        className={styles.menu}
        role="menu"
        aria-label="Image"
        style={{ left: at.x, top: at.y }}
        onMouseDown={(e) => e.preventDefault()}
        onContextMenu={(e) => e.preventDefault()}
      >
        <button
          type="button"
          role="menuitem"
          onClick={() => run(() => editImageText(editor, 'caption'))}
        >
          <span className={styles.icon}>T</span>
          {editor.getAttributes('image').title ? 'Edit caption…' : 'Add caption…'}
        </button>
        <button
          type="button"
          role="menuitem"
          onClick={() => run(() => editImageText(editor, 'alt'))}
        >
          <span className={styles.icon}>Aa</span>
          Alt text…
        </button>
        <div className={styles.separator} />
        <button type="button" role="menuitem" onClick={() => run(() => void clipboard('cut'))}>
          <span className={styles.icon} />
          Cut
        </button>
        <button type="button" role="menuitem" onClick={() => run(() => void clipboard('copy'))}>
          <span className={styles.icon} />
          Copy
        </button>
        <button
          type="button"
          role="menuitem"
          onClick={() => run(() => editor.chain().focus().deleteSelection().run())}
        >
          <span className={styles.icon} />
          Remove image
        </button>
      </div>
    ) : null

  const menu =
    imageMenu ??
    (at && editor ? (
      <div
        ref={ref}
        className={styles.menu}
        role="menu"
        aria-label="Formatting"
        style={{ left: at.x, top: at.y }}
        onMouseDown={(e) => e.preventDefault()}
        onContextMenu={(e) => e.preventDefault()}
      >
        <div className={styles.marks}>
          {textFormats.map((format) => (
            <button
              key={format.id}
              type="button"
              role="menuitemcheckbox"
              aria-checked={format.isActive(editor)}
              title={format.label}
              aria-label={format.label}
              className={format.isActive(editor) ? styles.active : undefined}
              onClick={() => run(() => format.apply(editor))}
            >
              {format.icon}
            </button>
          ))}
        </div>
        <div className={styles.separator} />
        {editor.isActive('table') && (
          <>
            <div className={styles.columns} role="group" aria-label="Table">
              {tableActions.map((action) => (
                <button
                  key={action.label}
                  type="button"
                  role="menuitem"
                  onClick={() => run(() => action.run(editor))}
                >
                  <span className={styles.icon}>{action.icon}</span>
                  {action.label}
                </button>
              ))}
            </div>
            <div className={styles.separator} />
          </>
        )}
        {blockFormats.map((format) => (
          <button
            key={format.id}
            type="button"
            role="menuitemradio"
            aria-checked={format.isActive(editor)}
            className={format.isActive(editor) ? styles.active : undefined}
            onClick={() => run(() => format.apply(editor))}
          >
            <span className={styles.icon}>{format.icon}</span>
            {format.label}
          </button>
        ))}
        <div className={styles.separator} />
        {insertFormats.map((format) => (
          <button
            key={format.id}
            type="button"
            role="menuitem"
            onClick={() => run(() => format.apply(editor))}
          >
            <span className={styles.icon}>{format.icon}</span>
            {format.label}
          </button>
        ))}
        {editor.isActive('link') && (
          <button type="button" role="menuitem" onClick={() => run(() => editLink(editor))}>
            <span className={styles.icon}>✎</span>
            Edit link…
          </button>
        )}
        {editor.isActive('link') && (
          <button
            type="button"
            role="menuitem"
            onClick={() =>
              run(() => editor.chain().focus().extendMarkRange('link').unsetLink().run())
            }
          >
            <span className={styles.icon}>⌀</span>
            Remove link
          </button>
        )}
        {section && (
          <button
            type="button"
            role="menuitem"
            onClick={() =>
              run(() => void copyLink(`${linkBase}#${encodeURIComponent(section.id)}`))
            }
          >
            <span className={styles.icon}>🔗</span>
            Copy link to section
          </button>
        )}
        <div className={styles.separator} />
        <button
          type="button"
          role="menuitem"
          disabled={!hasSelection}
          onClick={() => run(() => void clipboard('cut'))}
        >
          <span className={styles.icon} />
          Cut
        </button>
        <button
          type="button"
          role="menuitem"
          disabled={!hasSelection}
          onClick={() => run(() => void clipboard('copy'))}
        >
          <span className={styles.icon} />
          Copy
        </button>
        <button type="button" role="menuitem" onClick={() => run(() => void clipboard('paste'))}>
          <span className={styles.icon} />
          Paste
        </button>
        <p className={styles.footnote}>Shift + right-click for the browser menu</p>
      </div>
    ) : null)

  return { menu, onMouseDown, onContextMenu }
}
