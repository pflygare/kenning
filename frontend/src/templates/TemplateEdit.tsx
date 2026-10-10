import { EditorContent, useEditor } from '@tiptap/react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router'
import { apiGet, apiPost, apiPut, errorMessage } from '../api/client'
import type { TemplateDetail } from '../api/types/TemplateDetail'
import type { UpdateTemplateRequest } from '../api/types/UpdateTemplateRequest'
import Popover from '../components/Popover'
import { useContextMenu, useSlashMenu } from '../pages/EditorMenus'
import { extensions } from '../pages/editorExtensions'
import { ImageUpload } from '../pages/imageUpload'
import PageMeta from '../pages/PageMeta'
import editStyles from '../pages/PageEdit.module.css'
import Toolbar from '../pages/Toolbar'
import { useOrg } from '../routes/useOrg'
import styles from './TemplateEdit.module.css'

const AUTOSAVE_DELAY_MS = 800

type Fields = { name: string; description: string; title: string }

/** Edit a template. Loads it, then mounts the editor with its text. */
export default function TemplateEdit() {
  const org = useOrg()
  const { template: id = '' } = useParams()
  const api = `/orgs/${org.slug}/templates/${id}`
  const [template, setTemplate] = useState<TemplateDetail | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<TemplateDetail>(api)
      .then(setTemplate)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [api])

  if (error) return <p className="alert error">{error}</p>
  if (template?.id !== id) return <p className="status">Loading…</p>
  return <Editor key={id} org={org.slug} api={api} template={template} />
}

function Editor({ org, api, template }: { org: string; api: string; template: TemplateDetail }) {
  const navigate = useNavigate()
  const isNew = (useLocation().state as { isNew?: boolean } | null)?.isNew ?? false
  const [fields, setFields] = useState<Fields>({
    name: isNew ? '' : template.name,
    description: template.description,
    title: template.title,
  })
  const [saving, setSaving] = useState<'saved' | 'dirty' | 'saving' | 'error'>('saved')
  const [message, setMessage] = useState<string | null>(null)
  const [uploads, setUploads] = useState(0)

  const fieldsRef = useRef(fields)
  const pending = useRef(false)
  const inFlight = useRef<Promise<void> | null>(null)
  const timer = useRef<number | undefined>(undefined)
  const saveRef = useRef<() => Promise<void>>(async () => {})

  const editor = useEditor({
    extensions: [
      ...extensions('The text new pages start with. Type / for headings, lists and more…'),
      ImageUpload.configure({
        org,
        onUploading: (change) => setUploads((n) => n + change),
        onError: setMessage,
      }),
    ],
    content: template.body_md,
    contentType: 'markdown',
    onUpdate: () => schedule(),
  })
  const slashMenu = useSlashMenu(editor)
  const contextMenu = useContextMenu(editor)

  const save = useCallback(async () => {
    if (!editor || !pending.current) return
    if (inFlight.current) await inFlight.current
    pending.current = false
    setSaving('saving')
    const { name, description, title } = fieldsRef.current
    const body: UpdateTemplateRequest = {
      name: name.trim() || 'Untitled template',
      description,
      title,
      body_md: editor.getMarkdown(),
    }
    inFlight.current = apiPut<TemplateDetail>(api, body)
      .then(() => {
        setSaving(pending.current ? 'dirty' : 'saved')
        setMessage(null)
      })
      .catch((err: unknown) => {
        pending.current = true
        setSaving('error')
        setMessage(errorMessage(err))
      })
      .finally(() => {
        inFlight.current = null
      })
    await inFlight.current
  }, [api, editor])

  useEffect(() => {
    saveRef.current = save
  }, [save])

  function schedule() {
    pending.current = true
    setSaving('dirty')
    window.clearTimeout(timer.current)
    timer.current = window.setTimeout(() => void saveRef.current(), AUTOSAVE_DELAY_MS)
  }

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

  const change = (field: keyof Fields, value: string) => {
    const next = { ...fieldsRef.current, [field]: value }
    fieldsRef.current = next
    setFields(next)
    schedule()
  }

  const done = async () => {
    window.clearTimeout(timer.current)
    await save()
    if (!pending.current) navigate(`/${org}/templates`)
  }

  const archive = async () => {
    if (
      !window.confirm(
        `Archive the template "${fields.name || template.name}"? Pages made from it stay as they are.`,
      )
    )
      return
    window.clearTimeout(timer.current)
    pending.current = false
    try {
      await apiPost(`${api}/archive`)
      navigate(`/${org}/templates`, { replace: true })
    } catch (err) {
      setMessage(errorMessage(err))
    }
  }

  const status =
    uploads > 0
      ? 'Uploading image…'
      : { saved: 'Saved', dirty: 'Unsaved changes', saving: 'Saving…', error: 'Not saved' }[saving]

  return (
    <div className={editStyles.editor}>
      <div className={editStyles.bar}>
        <span className={`muted ${editStyles.status}`} aria-live="polite">
          <span className={`${editStyles.dot} ${editStyles[uploads > 0 ? 'saving' : saving]}`} />{' '}
          {status}
        </span>
        <div className="row">
          <button className="primary" disabled={uploads > 0} onClick={() => void done()}>
            Done
          </button>
          <Popover label="⋯" ariaLabel="Template actions">
            {(close) => (
              <div role="menu">
                <button
                  className="menu-item danger"
                  role="menuitem"
                  onClick={() => (close(), void archive())}
                >
                  Archive template
                </button>
              </div>
            )}
          </Popover>
        </div>
      </div>
      {message && <p className="alert error">{message}</p>}

      <p className={styles.kicker}>Template</p>
      <input
        className={editStyles.title}
        aria-label="Template name"
        placeholder="Name this template"
        maxLength={100}
        autoFocus={isNew}
        value={fields.name}
        onChange={(e) => change('name', e.target.value)}
      />
      <input
        className={styles.description}
        aria-label="Description"
        placeholder="What it's for, shown when picking a template"
        maxLength={500}
        value={fields.description}
        onChange={(e) => change('description', e.target.value)}
      />

      <div className={`card ${styles.preset}`}>
        <label className={styles.field}>
          <span>New pages are titled</span>
          <input
            placeholder="Left to the writer"
            maxLength={200}
            value={fields.title}
            onChange={(e) => change('title', e.target.value)}
          />
        </label>
        <div className={styles.field}>
          <span>and start in</span>
          <PageMeta org={org} api={api} page={template} />
        </div>
        <p className={`muted ${styles.hint}`}>
          In the title and text, <code>{'{{date}}'}</code>, <code>{'{{author}}'}</code> and{' '}
          <code>{'{{title}}'}</code> become today's date, the writer's name and the page title.
        </p>
      </div>

      {editor && <Toolbar editor={editor} />}
      <div
        onKeyDownCapture={slashMenu.onKeyDown}
        onMouseDownCapture={contextMenu.onMouseDown}
        onContextMenu={contextMenu.onContextMenu}
      >
        <EditorContent editor={editor} className={`prose ${editStyles.body}`} />
      </div>
      {slashMenu.menu}
      {contextMenu.menu}
    </div>
  )
}
