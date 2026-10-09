/** The page list filtered to pages with this category value. */
export function valueListPath(org: string, value: { id: string }) {
  return `/${org}?value=${value.id}`
}
