import { useState } from 'react'
import { Link } from 'react-router'
import { apiGet, apiPut, errorMessage } from '../api/client'
import type { PageCategory } from '../api/types/PageCategory'
import type { SetPageCategoryRequest } from '../api/types/SetPageCategoryRequest'
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

/** The page's topics, tags and category values, editable in place. Changes apply at once, not on publish. */
export default function PageMeta<
  T extends { topics: TopicRef[]; tags: TagRef[]; categories?: PageCategory[] },
>({
  org,
  api,
  page,
  onChange,
}: {
  org: string
  /** The page's (or template's) API path; its topics and tags are set below it. */
  api: string
  page: T
  onChange?: (page: T) => void
}) {
  const topicsCtx = useTopics()
  const [topics, setTopics] = useState(page.topics)
  const [tags, setTags] = useState(page.tags)
  const [categories, setCategories] = useState(page.categories ?? [])
  const [error, setError] = useState<string | null>(null)

  const saveTopics = async (ids: string[]) => {
    // Show the change at once; the server's answer replaces it.
    const before = topics
    const known = topicsCtx.topics ?? []
    setTopics(known.filter((t) => ids.includes(t.id)))
    try {
      const body: SetPageTopicsRequest = { topic_ids: ids }
      const updated = await apiPut<T>(`${api}/topics`, body)
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
      const updated = await apiPut<T>(`${api}/tags`, body)
      setTags(updated.tags)
      setError(null)
      onChange?.(updated)
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  const saveValue = async (categoryId: string, valueId: string | null) => {
    try {
      const body: SetPageCategoryRequest = { category_id: categoryId, value_id: valueId }
      const updated = await apiPut<T>(`${api}/categories`, body)
      setCategories(updated.categories ?? [])
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
      {categories.length > 0 && (
        <div className="chips" aria-label="Categories">
          {categories.map((category) => (
            <CategoryPicker
              key={category.category_id}
              category={category}
              onPick={(valueId) => void saveValue(category.category_id, valueId)}
            />
          ))}
        </div>
      )}
      {error && <p className="alert error">{error}</p>}
    </div>
  )
}

/** "Information class: Internal"; opens the category's values. Required and unset shows as a warning. */
function CategoryPicker({
  category,
  onPick,
}: {
  category: PageCategory
  onPick: (valueId: string | null) => void
}) {
  const { value } = category
  const label = value ? (
    <span className={`tag value ${value.color}`}>
      <span className={styles.categoryName}>{category.name}:</span> {value.name}
    </span>
  ) : category.required ? (
    <span className="tag value missing" title="Required before publishing">
      {category.name}: choose
    </span>
  ) : (
    <span className={styles.optional}>+ {category.name}</span>
  )
  return (
    <Popover
      label={label}
      ariaLabel={`${category.name}: ${value?.name ?? 'not set'}`}
      triggerClass={`small ghost ${styles.categoryTrigger}`}
      align="left"
    >
      {(close) => (
        <div role="menu" aria-label={category.name}>
          <div className={`menu-label ${styles.pickerLabel}`}>
            {category.name}
            {category.required && ' · required'}
          </div>
          {category.options.map((option) => (
            <button
              key={option.id}
              className="menu-item"
              role="menuitemradio"
              aria-checked={option.id === value?.id}
              onClick={() => {
                close()
                if (option.id !== value?.id) onPick(option.id)
              }}
            >
              <span className={`tag value ${option.color}`}>{option.name}</span>
              {option.id === value?.id && <span className={styles.tick}>✓</span>}
            </button>
          ))}
          {value && (
            <>
              <div className="menu-separator" />
              <button className="menu-item" role="menuitem" onClick={() => (close(), onPick(null))}>
                Clear
              </button>
            </>
          )}
        </div>
      )}
    </Popover>
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
