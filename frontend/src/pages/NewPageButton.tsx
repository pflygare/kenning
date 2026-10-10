import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { apiGet } from '../api/client'
import type { TemplateSummary } from '../api/types/TemplateSummary'
import Popover from '../components/Popover'
import type { useAction } from '../hooks/useAction'
import { pagePath } from './format'
import styles from './NewPageButton.module.css'
import { startPage } from './startPage'

/** "+ New page": blank, or from one of the organization's templates when it has any. */
export default function NewPageButton({
  org,
  topicId,
  action,
  label = '+ New page',
  onCreated,
}: {
  org: string
  topicId?: string
  action: ReturnType<typeof useAction>
  label?: string
  onCreated?: () => void
}) {
  const navigate = useNavigate()
  const [templates, setTemplates] = useState<TemplateSummary[] | null>(null)

  useEffect(() => {
    apiGet<TemplateSummary[]>(`/orgs/${org}/templates`)
      .then(setTemplates)
      .catch(() => setTemplates([]))
  }, [org])

  const start = (template: TemplateSummary | null) =>
    action.run(async () => {
      const started = await startPage(org, template, topicId)
      if (!started) return
      onCreated?.()
      navigate(`${pagePath(org, started.page)}/edit`, { state: { isNew: !started.titled } })
    })

  if (!templates?.length) {
    return (
      <button className="primary" disabled={action.busy} onClick={() => void start(null)}>
        {label}
      </button>
    )
  }

  return (
    <Popover label={`${label} ▾`} triggerClass="primary" ariaLabel="New page">
      {(close) => (
        <div role="menu" className={styles.menu}>
          <button className="menu-item" role="menuitem" onClick={() => (close(), void start(null))}>
            Blank page
          </button>
          <div className="menu-separator" />
          <div className="menu-label">From a template</div>
          {templates.map((template) => (
            <button
              key={template.id}
              className={`menu-item ${styles.template}`}
              role="menuitem"
              onClick={() => (close(), void start(template))}
            >
              <span className={styles.name}>{template.name}</span>
              {template.description && (
                <span className={styles.description}>{template.description}</span>
              )}
            </button>
          ))}
          <div className="menu-separator" />
          <Link className={`menu-item ${styles.manage}`} to={`/${org}/templates`}>
            Manage templates
          </Link>
        </div>
      )}
    </Popover>
  )
}
