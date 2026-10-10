import { useCallback, useEffect, useState } from 'react'
import { Link, useLocation, useNavigate, useParams } from 'react-router'
import { apiGet, apiPatch, apiPost, errorMessage } from '../api/client'
import type { MoveTopicRequest } from '../api/types/MoveTopicRequest'
import type { Topic } from '../api/types/Topic'
import type { TopicDetail } from '../api/types/TopicDetail'
import type { UpdateTopicRequest } from '../api/types/UpdateTopicRequest'
import Popover from '../components/Popover'
import { useAction } from '../hooks/useAction'
import { shortIdFrom } from '../pages/format'
import NewPageButton from '../pages/NewPageButton'
import PageRows from '../pages/PageRows'
import { useOrg } from '../routes/useOrg'
import { useTopics } from './context'
import NewTopicForm from './NewTopicForm'
import { childrenOf, flatten, subtreeIds, topicPath } from './tree'
import styles from './TopicView.module.css'

/** A topic: where it sits, its sub-topics and its pages. */
export default function TopicRoute() {
  const { topic: ref = '' } = useParams()
  // A fresh view per topic, so nothing from the last one lingers.
  return <TopicView key={ref} topicRef={ref} />
}

function TopicView({ topicRef: ref }: { topicRef: string }) {
  const org = useOrg()
  const api = `/orgs/${org.slug}/topics/${shortIdFrom(ref)}`
  const navigate = useNavigate()
  const location = useLocation()
  const topicsCtx = useTopics()
  const [detail, setDetail] = useState<TopicDetail | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [editing, setEditing] = useState(false)
  const [adding, setAdding] = useState(false)
  const action = useAction()

  const load = useCallback(
    () =>
      apiGet<TopicDetail>(api)
        .then((d) => {
          setDetail(d)
          setError(null)
        })
        .catch((err: unknown) => setError(errorMessage(err))),
    [api],
  )

  useEffect(() => {
    void load()
  }, [load])

  // Keep the address in step with the name.
  useEffect(() => {
    if (detail && location.pathname !== topicPath(org.slug, detail.topic)) {
      navigate(topicPath(org.slug, detail.topic), { replace: true })
    }
  }, [detail, org.slug, location.pathname, navigate])

  if (error) return <p className="alert error">{error}</p>
  if (!detail) return <p className="status">Loading…</p>

  const { topic, path, children, pages } = detail
  const all = topicsCtx.topics ?? []
  const siblings = childrenOf(all, topic.parent_id)
  const index = siblings.findIndex((t) => t.id === topic.id)

  const move = (parentId: string | null, to: number) =>
    action.run(async () => {
      const body: MoveTopicRequest = { parent_id: parentId, index: to }
      topicsCtx.set(await apiPost<Topic[]>(`${api}/move`, body))
      await load()
    })

  const archive = () => {
    if (!window.confirm(`Archive "${topic.name}"? Its pages stay, but leave this topic.`)) return
    void action.run(async () => {
      await apiPost(`${api}/archive`)
      await topicsCtx.reload()
      navigate(path.length ? topicPath(org.slug, path[path.length - 1]) : `/${org.slug}`, {
        replace: true,
      })
    })
  }

  // Where it can move: anywhere but inside itself.
  const inside = subtreeIds(all, topic.id)
  const destinations = flatten(all).filter(({ topic: t }) => !inside.has(t.id))

  return (
    <div className={styles.topic}>
      <nav className={styles.crumbs} aria-label="Topic path">
        <Link to={`/${org.slug}`}>Pages</Link>
        {path.map((p) => (
          <span key={p.id}>
            <span className={styles.sep}>/</span>
            <Link to={topicPath(org.slug, p)}>{p.name}</Link>
          </span>
        ))}
      </nav>

      {editing ? (
        <EditTopic
          topic={topic}
          onCancel={() => setEditing(false)}
          onSave={async (body) => {
            setDetail(await apiPatch<TopicDetail>(api, body))
            setEditing(false)
            void topicsCtx.reload()
          }}
        />
      ) : (
        <header className={styles.header}>
          <div>
            <h1>{topic.name}</h1>
            {topic.description && <p className={styles.description}>{topic.description}</p>}
          </div>
          <div className="row">
            <NewPageButton
              org={org.slug}
              topicId={topic.id}
              action={action}
              onCreated={() => void topicsCtx.reload()}
            />
            <Popover label="⋯" ariaLabel="Topic actions">
              {(close) => (
                <div role="menu">
                  <button className="menu-item" role="menuitem" onClick={() => (close(), setEditing(true))}>
                    Rename and describe…
                  </button>
                  <button className="menu-item" role="menuitem" onClick={() => (close(), setAdding(true))}>
                    New sub-topic
                  </button>
                  <div className="menu-separator" />
                  <button
                    className="menu-item"
                    role="menuitem"
                    disabled={index <= 0 || action.busy}
                    onClick={() => (close(), void move(topic.parent_id, index - 1))}
                  >
                    Move up
                  </button>
                  <button
                    className="menu-item"
                    role="menuitem"
                    disabled={index < 0 || index >= siblings.length - 1 || action.busy}
                    onClick={() => (close(), void move(topic.parent_id, index + 1))}
                  >
                    Move down
                  </button>
                  <div className="menu-label">Move under</div>
                  <select
                    aria-label="Move under"
                    className={styles.moveSelect}
                    value={topic.parent_id ?? ''}
                    onChange={(e) => {
                      const parent = e.target.value || null
                      close()
                      void move(parent, childrenOf(all, parent).length)
                    }}
                  >
                    <option value="">Top level</option>
                    {destinations.map(({ topic: t, depth }) => (
                      <option key={t.id} value={t.id}>
                        {'  '.repeat(depth)}
                        {t.name}
                      </option>
                    ))}
                  </select>
                  <div className="menu-separator" />
                  <button className="menu-item danger" role="menuitem" onClick={() => (close(), archive())}>
                    Archive topic
                  </button>
                </div>
              )}
            </Popover>
          </div>
        </header>
      )}
      {action.error && <p className="alert error">{action.error}</p>}

      {(children.length > 0 || adding) && (
        <section className={styles.section}>
          <h2>Sub-topics</h2>
          <ul className={styles.children}>
            {children.map((child) => (
              <li key={child.id}>
                <Link to={topicPath(org.slug, child)} className={`card ${styles.child}`}>
                  <span className={styles.childName}>{child.name}</span>
                  <span className="muted">
                    {child.page_count} {child.page_count === 1 ? 'page' : 'pages'}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
          {adding && (
            <div className={styles.addForm}>
              <NewTopicForm org={org.slug} parentId={topic.id} onClose={() => setAdding(false)} />
            </div>
          )}
        </section>
      )}

      <section className={styles.section}>
        <h2>Pages</h2>
        {pages.length > 0 ? (
          <PageRows org={org.slug} pages={pages} />
        ) : (
          <div className="card empty">
            <p>
              No pages here yet. Write one with <strong>+ New page</strong>, or add an existing page
              from its <strong>Add to topic</strong> menu.
            </p>
          </div>
        )}
      </section>
    </div>
  )
}

function EditTopic({
  topic,
  onSave,
  onCancel,
}: {
  topic: Topic
  onSave: (body: UpdateTopicRequest) => Promise<void>
  onCancel: () => void
}) {
  const [name, setName] = useState(topic.name)
  const [description, setDescription] = useState(topic.description)
  const action = useAction()
  return (
    <form
      className={`card ${styles.edit}`}
      onSubmit={(e) => {
        e.preventDefault()
        void action.run(() => onSave({ name, description }))
      }}
    >
      <label>
        Name
        <input autoFocus maxLength={100} value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <label>
        Description <span className="hint">optional</span>
        <textarea
          rows={2}
          maxLength={1000}
          value={description}
          placeholder="What belongs in this topic?"
          onChange={(e) => setDescription(e.target.value)}
        />
      </label>
      {action.error && <p className="alert error">{action.error}</p>}
      <div className="row">
        <button className="primary" disabled={action.busy}>
          Save
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  )
}
