import { useEffect, useState } from 'react'
import { apiGet, errorMessage } from '../api/client'
import type { PageSummary } from '../api/types/PageSummary'
import { useOrg } from '../routes/useOrg'
import PageRows from './PageRows'

/** Pages with unpublished changes that you saved last. */
export default function MyDrafts() {
  const org = useOrg()
  const [pages, setPages] = useState<PageSummary[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    apiGet<PageSummary[]>(`/orgs/${org.slug}/pages?my_drafts=true`)
      .then(setPages)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [org.slug])

  return (
    <>
      <div className="page-header">
        <div>
          <h1>My drafts</h1>
          <p>Pages with changes you saved that aren't published yet.</p>
        </div>
      </div>
      {error && <p className="alert error">{error}</p>}
      {pages?.length === 0 && <p className="muted">Nothing waiting. Everything you wrote is published.</p>}
      {pages && pages.length > 0 && <PageRows org={org.slug} pages={pages} />}
    </>
  )
}
