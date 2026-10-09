import { useCallback, useEffect, useState } from 'react'
import { Link } from 'react-router'
import { apiDelete, apiGet, apiPatch, apiPost, errorMessage } from '../api/client'
import type { CreateTagRequest } from '../api/types/CreateTagRequest'
import type { MergeTagRequest } from '../api/types/MergeTagRequest'
import type { Tag } from '../api/types/Tag'
import type { TagColor } from '../api/types/TagColor'
import type { UpdateTagRequest } from '../api/types/UpdateTagRequest'
import Popover from '../components/Popover'
import { useAction } from '../hooks/useAction'
import { useOrg } from '../routes/useOrg'
import ColorPicker from './ColorPicker'
import { tagListPath } from './colors'
import styles from './TagsPage.module.css'

/** Every tag in the organization: create, rename, recolor, merge and delete. */
export default function TagsPage() {
  const org = useOrg()
  const api = `/orgs/${org.slug}/tags`
  const [tags, setTags] = useState<Tag[] | null>(null)
  const [name, setName] = useState('')
  const [color, setColor] = useState<TagColor>('blue')
  const [renaming, setRenaming] = useState<string | null>(null)
  const action = useAction()

  const { setError } = action
  const load = useCallback(
    () =>
      apiGet<Tag[]>(api)
        .then(setTags)
        .catch((err: unknown) => setError(errorMessage(err))),
    [api, setError],
  )

  useEffect(() => {
    void load()
  }, [load])

  const create = () =>
    action.run(async () => {
      const body: CreateTagRequest = { name, color }
      await apiPost(api, body)
      setName('')
      await load()
    })

  const update = (tag: Tag, body: UpdateTagRequest) =>
    action.run(async () => {
      await apiPatch(`${api}/${tag.id}`, body)
      setRenaming(null)
      await load()
    })

  const merge = (tag: Tag, into: Tag) => {
    const pages = tag.page_count === 1 ? '1 page' : `${tag.page_count} pages`
    if (!window.confirm(`Merge "${tag.name}" into "${into.name}"? ${pages} get "${into.name}" and "${tag.name}" is deleted.`))
      return
    void action.run(async () => {
      const body: MergeTagRequest = { into_id: into.id }
      await apiPost(`${api}/${tag.id}/merge`, body)
      await load()
    })
  }

  const remove = (tag: Tag) => {
    const pages = tag.page_count === 1 ? '1 page' : `${tag.page_count} pages`
    if (!window.confirm(`Delete "${tag.name}"? It comes off ${pages}.`)) return
    void action.run(async () => {
      await apiDelete(`${api}/${tag.id}`)
      await load()
    })
  }

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Tags</h1>
          <p>Labels for pages. Add them from any page; manage them here.</p>
        </div>
      </div>

      <form
        className={`card ${styles.create}`}
        onSubmit={(e) => {
          e.preventDefault()
          void create()
        }}
      >
        <ColorPicker value={color} onChange={setColor} />
        <input
          aria-label="New tag name"
          placeholder="New tag"
          maxLength={50}
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <button className="primary" disabled={action.busy || !name.trim()}>
          Add tag
        </button>
      </form>
      {action.error && <p className="alert error">{action.error}</p>}

      {tags?.length === 0 && <p className="muted">No tags yet.</p>}
      {tags && tags.length > 0 && (
        <ul className={`card ${styles.list}`}>
          {tags.map((tag) => (
            <li key={tag.id} className={styles.row}>
              <ColorPicker value={tag.color} onChange={(c) => void update(tag, { color: c })} />
              {renaming === tag.id ? (
                <RenameForm
                  tag={tag}
                  onSave={(newName) => void update(tag, { name: newName })}
                  onCancel={() => setRenaming(null)}
                />
              ) : (
                <Link to={tagListPath(org.slug, tag)} className={`tag ${tag.color}`}>
                  {tag.name}
                </Link>
              )}
              <span className={`muted ${styles.count}`}>
                {tag.page_count === 1 ? '1 page' : `${tag.page_count} pages`}
              </span>
              <Popover label="⋯" ariaLabel={`Actions for ${tag.name}`} triggerClass="small ghost">
                {(close) => (
                  <div role="menu">
                    <button className="menu-item" role="menuitem" onClick={() => (close(), setRenaming(tag.id))}>
                      Rename
                    </button>
                    {tags.length > 1 && (
                      <>
                        <div className="menu-label">Merge into</div>
                        {tags
                          .filter((t) => t.id !== tag.id)
                          .map((into) => (
                            <button
                              key={into.id}
                              className="menu-item"
                              role="menuitem"
                              onClick={() => (close(), merge(tag, into))}
                            >
                              <span className={`tag ${into.color}`}>{into.name}</span>
                            </button>
                          ))}
                      </>
                    )}
                    <div className="menu-separator" />
                    <button className="menu-item danger" role="menuitem" onClick={() => (close(), remove(tag))}>
                      Delete tag
                    </button>
                  </div>
                )}
              </Popover>
            </li>
          ))}
        </ul>
      )}
    </>
  )
}

function RenameForm({
  tag,
  onSave,
  onCancel,
}: {
  tag: Tag
  onSave: (name: string) => void
  onCancel: () => void
}) {
  const [name, setName] = useState(tag.name)
  return (
    <form
      className={styles.rename}
      onSubmit={(e) => {
        e.preventDefault()
        if (name.trim() && name.trim() !== tag.name) onSave(name)
        else onCancel()
      }}
    >
      <input
        autoFocus
        aria-label={`Rename ${tag.name}`}
        maxLength={50}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
      />
      <button className="small primary">Save</button>
      <button type="button" className="small" onClick={onCancel}>
        Cancel
      </button>
    </form>
  )
}
