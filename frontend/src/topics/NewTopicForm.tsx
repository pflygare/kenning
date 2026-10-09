import { useState } from 'react'
import { useNavigate } from 'react-router'
import { apiPost, errorMessage } from '../api/client'
import type { CreateTopicRequest } from '../api/types/CreateTopicRequest'
import type { TopicDetail } from '../api/types/TopicDetail'
import { useTopics } from './context'
import styles from './NewTopicForm.module.css'
import { topicPath } from './tree'

/** A one-line form that creates a topic and opens it. */
export default function NewTopicForm({
  org,
  parentId,
  onClose,
  compact = false,
}: {
  org: string
  parentId: string | null
  onClose: () => void
  compact?: boolean
}) {
  const navigate = useNavigate()
  const { reload } = useTopics()
  const [name, setName] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const submit = async () => {
    if (!name.trim()) return onClose()
    setBusy(true)
    try {
      const body: CreateTopicRequest = { name, description: '', parent_id: parentId }
      const created = await apiPost<TopicDetail>(`/orgs/${org}/topics`, body)
      await reload()
      onClose()
      navigate(topicPath(org, created.topic))
    } catch (err) {
      setError(errorMessage(err))
      setBusy(false)
    }
  }

  return (
    <form
      className={compact ? styles.compact : styles.form}
      onSubmit={(e) => {
        e.preventDefault()
        void submit()
      }}
    >
      <input
        autoFocus
        aria-label={parentId ? 'Sub-topic name' : 'Topic name'}
        placeholder={parentId ? 'Sub-topic name' : 'Topic name'}
        maxLength={100}
        value={name}
        disabled={busy}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === 'Escape' && onClose()}
        onBlur={() => !name.trim() && onClose()}
      />
      {!compact && (
        <>
          <button className="primary" disabled={busy}>
            Create
          </button>
          <button type="button" onClick={onClose}>
            Cancel
          </button>
        </>
      )}
      {error && <p className="alert error">{error}</p>}
    </form>
  )
}
