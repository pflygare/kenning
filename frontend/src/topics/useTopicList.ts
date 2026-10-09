import { useCallback, useEffect, useState } from 'react'
import { apiGet } from '../api/client'
import type { Topic } from '../api/types/Topic'
import type { Topics } from './context'

/** Loads an organization's topics. They are kept with their organization, so switching shows none until loaded. */
export function useTopicList(org: string): Topics {
  const [loaded, setLoaded] = useState<{ org: string; topics: Topic[] } | null>(null)

  const reload = useCallback(
    () =>
      apiGet<Topic[]>(`/orgs/${org}/topics`)
        .then((topics) => setLoaded({ org, topics }))
        .catch(() => setLoaded((prev) => prev ?? { org, topics: [] })),
    [org],
  )

  useEffect(() => {
    void reload()
  }, [reload])

  return {
    topics: loaded?.org === org ? loaded.topics : null,
    reload,
    set: (topics) => setLoaded({ org, topics }),
  }
}
