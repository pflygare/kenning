import type { Topic } from '../api/types/Topic'

export function topicPath(org: string, topic: { slug: string; short_id: string }) {
  return `/${org}/t/${topic.slug}-${topic.short_id}`
}

/** Topics under `parentId` (null for the top level), in order. */
export function childrenOf(topics: Topic[], parentId: string | null) {
  return topics.filter((t) => t.parent_id === parentId)
}

/** Every topic depth-first, with its depth, for indented pickers. */
export function flatten(topics: Topic[], parentId: string | null = null, depth = 0) {
  const out: { topic: Topic; depth: number }[] = []
  for (const topic of childrenOf(topics, parentId)) {
    out.push({ topic, depth })
    out.push(...flatten(topics, topic.id, depth + 1))
  }
  return out
}

/** The topic's id and the ids of everything below it. */
export function subtreeIds(topics: Topic[], id: string): Set<string> {
  const ids = new Set([id])
  let grew = true
  while (grew) {
    grew = false
    for (const t of topics) {
      if (t.parent_id && ids.has(t.parent_id) && !ids.has(t.id)) {
        ids.add(t.id)
        grew = true
      }
    }
  }
  return ids
}

/** Ids of the topic's ancestors, nearest first. */
export function ancestorIds(topics: Topic[], id: string): string[] {
  const byId = new Map(topics.map((t) => [t.id, t]))
  const out: string[] = []
  let parent = byId.get(id)?.parent_id
  while (parent && !out.includes(parent)) {
    out.push(parent)
    parent = byId.get(parent)?.parent_id
  }
  return out
}
