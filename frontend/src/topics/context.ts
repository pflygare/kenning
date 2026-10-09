import { createContext, useContext } from 'react'
import type { Topic } from '../api/types/Topic'

export type Topics = {
  /** Every topic in the organization, siblings in order; null while loading. */
  topics: Topic[] | null
  reload: () => Promise<void>
  /** Replace the list with one the server just returned. */
  set: (topics: Topic[]) => void
}

export const TopicsContext = createContext<Topics | null>(null)

/** The organization's topics, loaded once by `OrgLayout`. */
export function useTopics(): Topics {
  const topics = useContext(TopicsContext)
  if (!topics) throw new Error('useTopics outside OrgLayout')
  return topics
}
