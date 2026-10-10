import { useState } from 'react'
import type { Topic } from '../api/types/Topic'
import { ancestorIds, childrenOf, flatten } from './tree'
import styles from './TopicChecklist.module.css'

/** How far each level is indented, in rem: enough for a checkbox and its connector. */
const INDENT = 1.5

/**
 * Topics to tick, one per line, with a search field on top. The whole tree shows
 * indented, with lines joining sub-topics to their parent; typing narrows it to
 * matching topics, each with its parent path.
 */
export default function TopicChecklist({
  topics,
  checked,
  onToggle,
  label,
}: {
  topics: Topic[]
  checked: string[]
  onToggle: (id: string) => void
  label: string
}) {
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const byId = new Map(topics.map((t) => [t.id, t]))
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean)
  const rows = flatten(topics).filter(
    ({ topic }) => !words.length || words.every((w) => topic.name.toLowerCase().includes(w)),
  )
  // A topic is its parent's last child when no sibling follows it; the lines stop there.
  const isLast = (topic: Topic) => childrenOf(topics, topic.parent_id).at(-1)?.id === topic.id
  /** The tree lines left of a topic at `depth`: one column per level above it. */
  const guides = (topic: Topic, depth: number) => {
    const chain = [topic, ...ancestorIds(topics, topic.id).map((id) => byId.get(id)!)]
    // chain[k] sits at depth (depth - k); column c holds the line for the level-c+1 node.
    return Array.from({ length: depth }, (_, column) => {
      const node = chain[depth - 1 - column]
      if (column === depth - 1) return isLast(node) ? 'elbow' : 'tee'
      return isLast(node) ? 'blank' : 'line'
    })
  }
  const path = (topic: Topic) =>
    ancestorIds(topics, topic.id)
      .reverse()
      .map((id) => byId.get(id)?.name)
      .join(' / ')

  if (topics.length === 0) {
    return (
      <p className={`muted ${styles.empty}`}>
        No topics yet. Create one with + next to Topics in the sidebar.
      </p>
    )
  }

  return (
    <div className={styles.picker}>
      <input
        className={styles.search}
        type="search"
        placeholder="Find a topic"
        aria-label="Find a topic"
        autoFocus
        value={query}
        onChange={(e) => {
          setQuery(e.target.value)
          setActive(0)
        }}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown') setActive((i) => Math.min(i + 1, rows.length - 1))
          else if (e.key === 'ArrowUp') setActive((i) => Math.max(i - 1, 0))
          else if (e.key === 'Enter' && rows[active]) onToggle(rows[active].topic.id)
          else return
          e.preventDefault()
        }}
      />
      <div className={styles.list} role="group" aria-label={label}>
        {rows.map(({ topic, depth }, i) => (
          <label
            key={topic.id}
            className={`${styles.row} ${i === active && words.length ? styles.active : ''}`}
            style={{ paddingLeft: `${0.625 + (words.length ? 0 : depth) * INDENT}rem` }}
            onMouseEnter={() => setActive(i)}
          >
            {!words.length &&
              guides(topic, depth).map((kind, column) => (
                <span
                  key={column}
                  className={`${styles.guide} ${styles[kind]}`}
                  style={{ left: `${0.625 + column * INDENT}rem` }}
                  aria-hidden="true"
                />
              ))}
            <input
              type="checkbox"
              checked={checked.includes(topic.id)}
              onChange={() => onToggle(topic.id)}
            />
            <span className={styles.name}>
              {topic.name}
              {words.length > 0 && topic.parent_id && (
                <span className={styles.path}>{path(topic)}</span>
              )}
            </span>
          </label>
        ))}
        {rows.length === 0 && (
          <p className={`muted ${styles.empty}`}>No topic matches “{query}”.</p>
        )}
      </div>
    </div>
  )
}
