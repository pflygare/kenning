import { useState } from 'react'
import { Link } from 'react-router'
import { apiGet, apiPut, errorMessage } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import type { SetPageTagsRequest } from '../api/types/SetPageTagsRequest'
import type { SetPageTopicsRequest } from '../api/types/SetPageTopicsRequest'
import type { Tag } from '../api/types/Tag'
import type { TagRef } from '../api/types/TagRef'
import type { TopicRef } from '../api/types/TopicRef'
import Popover from '../components/Popover'
import TagChip from '../tags/TagChip'
import { useTopics } from '../topics/context'
import { flatten, topicPath } from '../topics/tree'
import styles from './PageMeta.module.css'

/** The page's topics and tags, editable in place. Changes apply at once, not on publish. */
export default function PageMeta({
  org,
  api,
  page,
  onChange,
}: {
  org: string
  api: string
  page: { topics: TopicRef[]; tags: TagRef[] }
  onChange?: (page: PageDetail) => void
}) {
  const topicsCtx = useTopics()
  const [topics, setTopics] = useState(page.topics)
  const [tags, setTags] = useState(page.tags)
  const [error, setError] = useState<string | null>(null)

  const saveTopics = async (ids: string[]) => {
    // Show the change at once; the server's answer replaces it.
    const before = topics
    const known = topicsCtx.topics ?? []
    setTopics(known.filter((t) => ids.includes(t.id)))
    try {
      const body: SetPageTopicsRequest = { topic_ids: ids }
      const updated = await apiPut<PageDetail>(`${api}/topics`, body)
      setTopics(updated.topics)
      setError(null)
      onChange?.(updated)
      void topicsCtx.reload()
    } catch (err) {
      setTopics(before)
      setError(errorMessage(err))
    }
  }

  const saveTags = async (names: string[]) => {
    try {
      const body: SetPageTagsRequest = { names }
      const updated = await apiPut<PageDetail>(`${api}/tags`, body)
      setTags(updated.tags)
      setError(null)
      onChange?.(updated)
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  const topicIds = topics.map((t) => t.id)
  const toggleTopic = (id: string) =>
    void saveTopics(topicIds.includes(id) ? topicIds.filter((t) => t !== id) : [...topicIds, id])

  return (
    <div className={styles.meta}>
      <div className="chips" aria-label="Topics">
        {topics.map((topic) => (
          <Link key={topic.id} className={styles.topic} to={topicPath(org, topic)}>
            <FolderIcon />
            {topic.name}
          </Link>
        ))}
        <Popover
          label={topics.length ? 'Topics…' : '+ Add to topic'}
          triggerClass={`small ghost ${styles.add}`}
          align="left"
        >
          {() => (
            <div role="group" aria-label="Topics for this page">
              {topicsCtx.topics?.length === 0 && (
                <p className={`muted ${styles.empty}`}>
                  No topics yet. Create one with + next to Topics in the sidebar.
                </p>
              )}
              {flatten(topicsCtx.topics ?? []).map(({ topic, depth }) => (
                <label
                  key={topic.id}
                  className={`menu-item ${styles.check}`}
                  style={{ paddingLeft: `${0.625 + depth * 1}rem` }}
                >
                  <input
                    type="checkbox"
                    checked={topicIds.includes(topic.id)}
                    onChange={() => toggleTopic(topic.id)}
                  />
                  {topic.name}
                </label>
              ))}
            </div>
          )}
        </Popover>
      </div>
      <div className="chips" aria-label="Tags">
        {tags.map((tag) => (
          <TagChip
            key={tag.id}
            org={org}
            tag={tag}
            onRemove={() => void saveTags(tags.filter((t) => t.id !== tag.id).map((t) => t.name))}
          />
        ))}
        <TagInput
          org={org}
          taken={tags.map((t) => t.slug)}
          onAdd={(name) => saveTags([...tags.map((t) => t.name), name])}
        />
      </div>
      {error && <p className="alert error">{error}</p>}
    </div>
  )
}

/** Same rule as the server: case and punctuation don't make a different tag. */
function slugify(name: string) {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

/** Type a tag name: existing tags are suggested, a new name creates a tag. */
function TagInput({
  org,
  taken,
  onAdd,
}: {
  org: string
  taken: string[]
  onAdd: (name: string) => Promise<void>
}) {
  const [all, setAll] = useState<Tag[] | null>(null)
  const [text, setText] = useState('')
  const [open, setOpen] = useState(false)
  const [index, setIndex] = useState(0)

  const load = () => {
    if (all) return
    apiGet<Tag[]>(`/orgs/${org}/tags`)
      .then(setAll)
      .catch(() => setAll([]))
  }

  const query = slugify(text)
  const matches = (all ?? [])
    .filter((t) => !taken.includes(t.slug) && (!query || t.slug.includes(query)))
    .slice(0, 8)
  const exact = (all ?? []).some((t) => t.slug === query)
  const options = [
    ...matches.map((t) => ({ name: t.name, label: t.name, tag: t as Tag | null })),
    ...(query && !exact && !taken.includes(query)
      ? [{ name: text.trim(), label: `Create “${text.trim()}”`, tag: null }]
      : []),
  ]

  const add = async (name: string) => {
    setText('')
    setIndex(0)
    await onAdd(name)
    setAll(null)
  }

  return (
    <div className={styles.tagInput}>
      <input
        aria-label="Add a tag"
        placeholder="+ Add tag"
        maxLength={50}
        value={text}
        onFocus={() => {
          load()
          setOpen(true)
        }}
        onBlur={() => setOpen(false)}
        onChange={(e) => {
          setText(e.target.value)
          setIndex(0)
          setOpen(true)
          load()
        }}
        onKeyDown={(e) => {
          if (e.key === 'ArrowDown') setIndex((i) => Math.min(i + 1, options.length - 1))
          else if (e.key === 'ArrowUp') setIndex((i) => Math.max(i - 1, 0))
          else if (e.key === 'Enter' || e.key === ',') {
            const choice = options[index]
            if (choice) void add(choice.name)
          } else if (e.key === 'Escape') {
            setText('')
            ;(e.target as HTMLInputElement).blur()
          } else return
          e.preventDefault()
        }}
      />
      {open && options.length > 0 && (
        <div className="popover-panel left" role="listbox" onMouseDown={(e) => e.preventDefault()}>
          {options.map((option, i) => (
            <button
              key={option.label}
              type="button"
              role="option"
              aria-selected={i === index}
              className={`menu-item ${i === index ? styles.selected : ''}`}
              onMouseEnter={() => setIndex(i)}
              onClick={() => void add(option.name)}
            >
              {option.tag ? <span className={`tag ${option.tag.color}`}>{option.label}</span> : option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function FolderIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 6a1 1 0 0 1 1-1h5l2 2h9a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z" />
    </svg>
  )
}
