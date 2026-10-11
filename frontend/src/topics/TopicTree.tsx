import { useState } from 'react'
import { NavLink, useLocation } from 'react-router'
import FolderIcon from '../components/FolderIcon'
import type { Topic } from '../api/types/Topic'
import { shortIdFrom } from '../pages/format'
import NewTopicForm from './NewTopicForm'
import styles from './TopicTree.module.css'
import { ancestorIds, childrenOf, topicPath } from './tree'

/**
 * The sidebar's topic tree. Branches open to show the topic being viewed. With `filter`,
 * only topics whose names contain it are shown, with the branches leading to them open.
 * Each row has a + that adds a sub-topic in place.
 */
export default function TopicTree({
  org,
  topics,
  filter = '',
}: {
  org: string
  topics: Topic[]
  filter?: string
}) {
  const { pathname } = useLocation()
  const match = /\/t\/([^/]+)/.exec(pathname)
  const current = match ? topics.find((t) => t.short_id === shortIdFrom(match[1])) : undefined
  // Branches someone opened or closed; the rest open only on the way to the current topic.
  const [toggled, setToggled] = useState<Map<string, boolean>>(new Map())
  // The topic a sub-topic is being added to, whose branch stays open meanwhile.
  const [adding, setAdding] = useState<string | null>(null)
  const ancestors = current ? ancestorIds(topics, current.id) : []
  const words = filter.toLowerCase().split(/\s+/).filter(Boolean)
  const shown = words.length
    ? new Set(
        topics
          .filter((t) => words.every((w) => t.name.toLowerCase().includes(w)))
          .flatMap((t) => [t.id, ...ancestorIds(topics, t.id)]),
      )
    : null
  const visible = (parentId: string | null) =>
    childrenOf(topics, parentId).filter((t) => !shown || shown.has(t.id))
  const isOpen = (id: string) =>
    shown || adding === id ? true : (toggled.get(id) ?? ancestors.includes(id))
  const toggle = (id: string) => setToggled((prev) => new Map(prev).set(id, !isOpen(id)))

  const branch = (parentId: string | null, depth: number) => (
    <ul className={styles.list} role={depth === 0 ? 'tree' : 'group'}>
      {parentId && adding === parentId && (
        <li style={{ paddingLeft: `${depth * 0.875 + 1.25}rem` }}>
          <NewTopicForm org={org} parentId={parentId} compact onClose={() => setAdding(null)} />
        </li>
      )}
      {visible(parentId).map((topic) => {
        const kids = visible(topic.id).length > 0
        const expanded = isOpen(topic.id)
        return (
          <li key={topic.id} role="treeitem" aria-expanded={kids ? expanded : undefined}>
            <div className={styles.row} style={{ paddingLeft: `${depth * 0.875}rem` }}>
              {kids ? (
                <button
                  type="button"
                  className={styles.toggle}
                  aria-label={expanded ? `Collapse ${topic.name}` : `Expand ${topic.name}`}
                  onClick={() => toggle(topic.id)}
                >
                  <span className={expanded ? styles.down : undefined}>›</span>
                </button>
              ) : (
                <span className={styles.toggle} />
              )}
              <NavLink
                to={topicPath(org, topic)}
                className={({ isActive }) =>
                  isActive ? `${styles.link} ${styles.active}` : styles.link
                }
              >
                <FolderIcon className={styles.icon} />
                <span className={styles.name}>{topic.name}</span>
              </NavLink>
              <button
                type="button"
                className={styles.add}
                aria-label={`Add a sub-topic to ${topic.name}`}
                title="Add a sub-topic"
                onClick={() => {
                  setAdding(topic.id)
                  setToggled((prev) => new Map(prev).set(topic.id, true))
                }}
              >
                +
              </button>
            </div>
            {(kids || adding === topic.id) && expanded && branch(topic.id, depth + 1)}
          </li>
        )
      })}
    </ul>
  )

  if (shown?.size === 0) return <p className={styles.none}>No topics match.</p>
  return branch(null, 0)
}
