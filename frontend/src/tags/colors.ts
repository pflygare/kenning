import type { TagColor } from '../api/types/TagColor'

/** Matches the `.tag.<color>` hues in index.css. */
export const TAG_HUES: Record<TagColor, string> = {
  gray: '#64748b',
  blue: '#2563eb',
  green: '#16a34a',
  amber: '#d97706',
  red: '#dc2626',
  purple: '#9333ea',
  teal: '#0d9488',
  pink: '#db2777',
}

export function tagListPath(org: string, tag: { slug: string }) {
  return `/${org}?tag=${encodeURIComponent(tag.slug)}`
}
