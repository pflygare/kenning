import { useEffect, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { apiGet, apiPost, errorMessage } from '../api/client'
import type { CreateTemplateRequest } from '../api/types/CreateTemplateRequest'
import type { TemplateDetail } from '../api/types/TemplateDetail'
import type { TemplateSummary } from '../api/types/TemplateSummary'
import { useAction } from '../hooks/useAction'
import { pagePath, timeAgo } from '../pages/format'
import { startPage } from '../pages/startPage'
import { useOrg } from '../routes/useOrg'
import styles from './TemplatesPage.module.css'

/** The organization's templates: start a page from one, edit them, or add another. */
export default function TemplatesPage() {
  const org = useOrg()
  const navigate = useNavigate()
  const action = useAction()
  const [templates, setTemplates] = useState<TemplateSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<TemplateSummary[]>(`/orgs/${org.slug}/templates`)
      .then(setTemplates)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [org.slug])

  const create = () =>
    action.run(async () => {
      const body: CreateTemplateRequest = { name: 'Untitled template' }
      const template = await apiPost<TemplateDetail>(`/orgs/${org.slug}/templates`, body)
      navigate(`/${org.slug}/templates/${template.id}`, { state: { isNew: true } })
    })

  const use = (template: TemplateSummary) =>
    action.run(async () => {
      const started = await startPage(org.slug, template)
      if (started) {
        navigate(`${pagePath(org.slug, started.page)}/edit`, {
          state: { isNew: !started.titled },
        })
      }
    })

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Templates</h1>
          <p>
            Starting points for new pages. Pick one under <strong>+ New page</strong>.
          </p>
        </div>
        <button className="primary" disabled={action.busy} onClick={() => void create()}>
          + New template
        </button>
      </div>
      {(error || action.error) && <p className="alert error">{error ?? action.error}</p>}
      {templates?.length === 0 && (
        <div className="card empty">
          <h2>No templates yet</h2>
          <p>
            Make one for pages you write often, like meeting notes or how-to guides. You can also
            save any page as a template from its ⋯ menu.
          </p>
          <button className="primary" disabled={action.busy} onClick={() => void create()}>
            Make a template
          </button>
        </div>
      )}
      {templates && templates.length > 0 && (
        <ul className={`card ${styles.list}`}>
          {templates.map((template) => (
            <li key={template.id} className={styles.row}>
              <Link to={`/${org.slug}/templates/${template.id}`} className={styles.item}>
                <span className={styles.name}>{template.name}</span>
                {template.description && (
                  <span className={`muted ${styles.description}`}>{template.description}</span>
                )}
              </Link>
              <span className={`muted ${styles.meta}`}>
                {template.updated_by_name} · {timeAgo(template.updated_at)}
              </span>
              <button className="small" disabled={action.busy} onClick={() => void use(template)}>
                Use
              </button>
            </li>
          ))}
        </ul>
      )}
    </>
  )
}
