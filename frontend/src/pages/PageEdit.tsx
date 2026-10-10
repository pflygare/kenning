import { EditorContent, useEditor } from '@tiptap/react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { ApiError, apiPost, apiPut, errorMessage } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import type { PublishRequest } from '../api/types/PublishRequest'
import type { SaveDraftRequest } from '../api/types/SaveDraftRequest'
import type { SavedDraft } from '../api/types/SavedDraft'
import { useContextMenu, useSlashMenu } from './EditorMenus'
import { extensions } from './editorExtensions'
import { pagePath } from './format'
import { useHeadings } from './headings'
import { ImageUpload } from './imageUpload'
import Outline from './Outline'
import PageMeta from './PageMeta'
import styles from './PageEdit.module.css'
import Toolbar from './Toolbar'
import { usePage } from './usePage'

const AUTOSAVE_DELAY_MS = 800

type SaveState = 'saved' | 'dirty' | 'saving' | 'conflict' | 'error'

/** The editor. Waits for the page, then mounts the editor with its content. */
export default function PageEdit() {
  const { org, api, page, error } = usePage()
  if (error) return <p className="alert error">{error}</p>
  if (!page) return <p className="status">Loading…</p>
  return <Editor key={page.short_id} orgSlug={org.slug} api={api} page={page} />
}

function Editor({ orgSlug, api, page }: { orgSlug: string; api: string; page: PageDetail }) {
  const navigate = useNavigate()
  const isNew = (useLocation().state as { isNew?: boolean } | null)?.isNew ?? false
  const start = page.draft ?? page.published!
  const [title, setTitle] = useState(isNew ? '' : start.title)
  const [state, setState] = useState<SaveState>('saved')
  const [message, setMessage] = useState<string | null>(null)
  const [uploads, setUploads] = useState(0)

  // The revision the next save builds on, and whether edits are waiting.
  const base = useRef(start.revision_id)
  const titleRef = useRef(title)
  const pending = useRef(false)
  const saving = useRef<Promise<void> | null>(null)
  const timer = useRef<number | undefined>(undefined)
  // The editor keeps the first onUpdate it is given, so it calls through this.
  const saveRef = useRef<() => Promise<void>>(async () => {})

  const editor = useEditor({
    extensions: [
      ...extensions('Write something, or type / for headings, lists and more…'),
      ImageUpload.configure({
        org: orgSlug,
        onUploading: (change) => setUploads((n) => n + change),
        onError: setMessage,
      }),
    ],
    content: start.body_md,
    contentType: 'markdown',
    autofocus: isNew ? false : 'end',
    onUpdate: () => schedule(),
  })

  const headings = useHeadings(editor)
  const slashMenu = useSlashMenu(editor)
  const contextMenu = useContextMenu(editor)

  const save = useCallback(async () => {
    if (!editor || !pending.current) return
    if (saving.current) await saving.current
    pending.current = false
    setState('saving')
    const body: SaveDraftRequest = {
      base_revision_id: base.current,
      title: titleRef.current.trim() || 'Untitled',
      body_md: editor.getMarkdown(),
    }
    saving.current = apiPut<SavedDraft>(`${api}/draft`, body)
      .then((saved) => {
        base.current = saved.revision_id
        setState(pending.current ? 'dirty' : 'saved')
        setMessage(null)
      })
      .catch((err: unknown) => {
        if (err instanceof ApiError && err.code === 'stale_draft') {
          setState('conflict')
        } else {
          pending.current = true
          setState('error')
          setMessage(errorMessage(err))
        }
      })
      .finally(() => {
        saving.current = null
      })
    await saving.current
  }, [api, editor])

  useEffect(() => {
    saveRef.current = save
  }, [save])

  function schedule() {
    pending.current = true
    setState((s) => (s === 'conflict' ? s : 'dirty'))
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => void saveRef.current(), AUTOSAVE_DELAY_MS)
  }

  // Save what's left when leaving, and warn if the browser would drop it.
  useEffect(() => {
    const warn = (event: BeforeUnloadEvent) => {
      if (pending.current) event.preventDefault()
    }
    window.addEventListener('beforeunload', warn)
    return () => {
      window.removeEventListener('beforeunload', warn)
      window.clearTimeout(timer.current)
    }
  }, [])

  const flush = async () => {
    window.clearTimeout(timer.current)
    await save()
    if (saving.current) await saving.current
  }

  const done = async () => {
    await flush()
    if (!pending.current) navigate(pagePath(orgSlug, page))
  }

  const publish = async () => {
    await flush()
    if (pending.current || state === 'conflict') return
    try {
      const body: PublishRequest = { revision_id: base.current }
      const published = await apiPost<PageDetail>(`${api}/publish`, body)
      navigate(pagePath(orgSlug, published))
    } catch (err) {
      if (err instanceof ApiError && err.code === 'stale_draft') setState('conflict')
      else setMessage(errorMessage(err))
    }
  }

  const saveStatus = {
    saved: 'Saved',
    dirty: 'Unsaved changes',
    saving: 'Saving…',
    conflict: 'Not saved',
    error: 'Not saved',
  }[state]
  const status =
    uploads === 0 ? saveStatus : uploads === 1 ? 'Uploading image…' : `Uploading ${uploads} images…`

  return (
    <div className={styles.editor}>
      <div className={styles.bar}>
        <span className={`muted ${styles.status}`} aria-live="polite">
          <span className={`${styles.dot} ${styles[uploads > 0 ? 'saving' : state]}`} /> {status}
        </span>
        <div className="row">
          <button disabled={uploads > 0} onClick={() => void done()}>
            Done
          </button>
          <button
            className="primary"
            disabled={state === 'conflict' || uploads > 0}
            onClick={() => void publish()}
          >
            Publish
          </button>
        </div>
      </div>

      {state === 'conflict' && (
        <div className="alert error">
          Someone else changed this page while you were editing, so your latest changes were not
          saved. Copy anything you need, then{' '}
          <Link to={pagePath(orgSlug, page)} reloadDocument>
            reload the page
          </Link>
          .
        </div>
      )}
      {message && <p className="alert error">{message}</p>}

      <PageMeta org={orgSlug} api={api} page={page} parts={['tags']} className={styles.tags} />
      <input
        className={styles.title}
        aria-label="Title"
        placeholder="Untitled"
        maxLength={200}
        autoFocus={isNew}
        value={title}
        onChange={(e) => {
          setTitle(e.target.value)
          titleRef.current = e.target.value
          schedule()
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && editor) {
            e.preventDefault()
            // Move focus now, not on the next frame, so fast typing lands in the body.
            editor.commands.setTextSelection(1)
            editor.view.focus()
          }
        }}
      />
      <PageMeta org={orgSlug} api={api} page={page} parts={['topics', 'categories']} />
      {editor && <Toolbar editor={editor} />}
      <div
        onKeyDownCapture={slashMenu.onKeyDown}
        onMouseDownCapture={contextMenu.onMouseDown}
        onContextMenu={contextMenu.onContextMenu}
      >
        <EditorContent editor={editor} className={`prose ${styles.body}`} />
      </div>
      {slashMenu.menu}
      {contextMenu.menu}
      <Outline headings={headings} />
      <p className="muted" style={{ marginTop: '2rem' }}>
        <Link to={`/${orgSlug}`}>← All pages</Link>
      </p>
    </div>
  )
}
