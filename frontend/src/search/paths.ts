export function searchPath(org: string, q: string) {
  return `/${org}/search?q=${encodeURIComponent(q)}`
}
