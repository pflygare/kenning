import { useCallback, useEffect, useState } from 'react'
import { useParams } from 'react-router'
import { apiGet, errorMessage } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import { useOrg } from '../routes/useOrg'
import { shortIdFrom } from './format'

/** Loads the page named in the URL; `api` is its base path in the API. */
export function usePage() {
  const org = useOrg()
  const { page: ref = '' } = useParams()
  const shortId = shortIdFrom(ref)
  const api = `/orgs/${org.slug}/pages/${shortId}`
  const [page, setPage] = useState<PageDetail | null>(null)
  const [error, setError] = useState<string | null>(null)

  const reload = useCallback(
    () =>
      apiGet<PageDetail>(api)
        .then((page) => {
          setPage(page)
          setError(null)
        })
        .catch((err: unknown) => setError(errorMessage(err))),
    [api],
  )

  useEffect(() => {
    void reload()
  }, [reload])

  // Following a link to another page keeps this hook; until that page arrives, show none.
  const current = page?.short_id === shortId ? page : null
  return { org, api, page: current, setPage, error, reload }
}
