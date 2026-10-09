import { useState } from 'react'
import { NavLink, useLocation } from 'react-router'
import type { Topic } from '../api/types/Topic'
import { shortIdFrom } from '../pages/format'
import styles from './TopicTree.module.css'
import { ancestorIds, childrenOf, topicPath } from './tree'

/** The sidebar's topic tree. Branches open to show the topic being viewed. */
export default function TopicTree({ org, topics }: { org: string; topics: Topic[] }) {
  const { pathname } = useLocation()
  const match = /\/t\/([^/]+)/.exec(pathname)
  const current = match ? topics.find((t) => t.short_id === shortIdFrom(match[1])) : undefined
  // Branches someone opened or closed; the rest open only on the way to the current topic.
  const [toggled, setToggled] = useState<Map<string, boolean>>(new Map())
  const ancestors = current ? ancestorIds(topics, current.id) : []
  const isOpen = (id: string) => toggled.get(id) ?? ancestors.includes(id)
  const toggle = (id: string) => setToggled((prev) => new Map(prev).set(id, !isOpen(id)))

  const branch = (parentId: string | null, depth: number) => (
    <ul className={styles.list} role={depth === 0 ? 'tree' : 'group'}>
      {childrenOf(topics, parentId).map((topic) => {
        const kids = childrenOf(topics, topic.id).length > 0
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
                className={({ isActive }) => (isActive ? `${styles.link} ${styles.active}` : styles.link)}
              >
                {topic.name}
              </NavLink>
            </div>
            {kids && expanded && branch(topic.id, depth + 1)}
          </li>
        )
      })}
    </ul>
  )

  return branch(null, 0)
}
