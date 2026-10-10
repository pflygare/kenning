/** The 10-character id at the end of a `/p/{slug}-{id}` path segment. */
export function shortIdFrom(ref: string): string {
  return ref.slice(-10)
}

export function pagePath(org: string, page: { slug: string; short_id: string }): string {
  return `/${org}/p/${page.slug}-${page.short_id}`
}

/** "just now", "5 minutes ago", "3 days ago", or a date. */
export function timeAgo(iso: string): string {
  const seconds = (Date.now() - new Date(iso).getTime()) / 1000
  if (seconds < 60) return 'just now'
  const units: [number, string][] = [
    [60 * 60, 'minute'],
    [60 * 60 * 24, 'hour'],
    [60 * 60 * 24 * 7, 'day'],
  ]
  let size = 60
  for (const [limit, unit] of units) {
    if (seconds < limit) {
      const n = Math.floor(seconds / size)
      return `${n} ${unit}${n === 1 ? '' : 's'} ago`
    }
    size = limit
  }
  return new Date(iso).toLocaleDateString()
}

/** "10 Oct 2026, 16:59" in the reader's locale. */
export function dateTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

/** "Restored from the version of 3 Oct 2026, 09:12", for revisions brought back. */
export function restoredNote(revision: {
  restored_from: string | null
  restored_from_at: string | null
}): string | null {
  if (!revision.restored_from) return null
  return revision.restored_from_at
    ? `Restored from the version of ${dateTime(revision.restored_from_at)}`
    : 'Restored from an earlier version'
}
