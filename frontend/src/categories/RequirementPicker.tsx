import { useState } from 'react'
import type { TopicRef } from '../api/types/TopicRef'
import Popover from '../components/Popover'
import { useTopics } from '../topics/context'
import { flatten } from '../topics/tree'
import styles from './CategoriesPage.module.css'

export type Requirement = { everywhere: boolean; topicIds: string[] }

/** Where a category must have a value: nowhere, on every page, or on pages in chosen topics. */
export default function RequirementPicker({
  value,
  onChange,
  label,
}: {
  value: Requirement
  onChange: (value: Requirement) => void
  label: string
}) {
  const { topics } = useTopics()
  // "In topics" with none ticked yet only exists here, not on the server.
  const [pickingTopics, setPickingTopics] = useState(false)
  const mode = value.everywhere
    ? 'all'
    : value.topicIds.length > 0 || pickingTopics
      ? 'topics'
      : 'none'
  const chosen = (topics ?? []).filter((t) => value.topicIds.includes(t.id))

  const toggle = (id: string) =>
    onChange({
      everywhere: false,
      topicIds: value.topicIds.includes(id)
        ? value.topicIds.filter((t) => t !== id)
        : [...value.topicIds, id],
    })

  return (
    <div className={styles.requirement}>
      <select
        aria-label={label}
        value={mode}
        onChange={(e) => {
          const next = e.target.value
          setPickingTopics(next === 'topics')
          if (next === 'all') onChange({ everywhere: true, topicIds: [] })
          else if (next === 'none') onChange({ everywhere: false, topicIds: [] })
          else if (value.everywhere) onChange({ everywhere: false, topicIds: value.topicIds })
        }}
      >
        <option value="none">Optional</option>
        <option value="all">Required on every page</option>
        <option value="topics">Required in some topics</option>
      </select>
      {mode === 'topics' && (
        <div className="chips">
          {chosen.map((topic) => (
            <TopicChip key={topic.id} topic={topic} onRemove={() => toggle(topic.id)} />
          ))}
          <Popover
            label={chosen.length ? 'Topics…' : '+ Choose topics'}
            triggerClass="small ghost"
            align="left"
          >
            {() => (
              <div role="group" aria-label="Topics where it's required">
                {topics?.length === 0 && (
                  <p className={`muted ${styles.empty}`}>
                    No topics yet. Create one with + next to Topics in the sidebar.
                  </p>
                )}
                {flatten(topics ?? []).map(({ topic, depth }) => (
                  <label
                    key={topic.id}
                    className={`menu-item ${styles.check}`}
                    style={{ paddingLeft: `${0.625 + depth}rem` }}
                  >
                    <input
                      type="checkbox"
                      checked={value.topicIds.includes(topic.id)}
                      onChange={() => toggle(topic.id)}
                    />
                    {topic.name}
                  </label>
                ))}
              </div>
            )}
          </Popover>
          {chosen.length > 0 && <span className="hint">and their sub-topics</span>}
        </div>
      )}
    </div>
  )
}

function TopicChip({ topic, onRemove }: { topic: TopicRef; onRemove: () => void }) {
  return (
    <span className="topic-chip">
      {topic.name}
      <button type="button" aria-label={`Not required in ${topic.name}`} onClick={onRemove}>
        ×
      </button>
    </span>
  )
}
