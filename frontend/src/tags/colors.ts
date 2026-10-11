import type { CSSProperties } from 'react'
import type { TagColor } from '../api/types/TagColor'

/**
 * The palette, in the order new tags and values take colors (as the server's `colors::PALETTE`):
 * each next to ones that look unlike it. Labels derive their fill and text from the hue, so
 * both themes work.
 */
export const PALETTE: [TagColor, string][] = [
  ['blue', '#2563eb'],
  ['green', '#16a34a'],
  ['amber', '#d97706'],
  ['red', '#dc2626'],
  ['purple', '#9333ea'],
  ['teal', '#0d9488'],
  ['pink', '#db2777'],
  ['orange', '#ea580c'],
  ['indigo', '#4f46e5'],
  ['lime', '#65a30d'],
  ['cyan', '#0891b2'],
  ['fuchsia', '#c026d3'],
  ['yellow', '#ca8a04'],
  ['sky', '#0284c7'],
  ['brown', '#92400e'],
  ['gray', '#64748b'],
]

const HUES = new Map(PALETTE)

/** A color's hue: the palette's for a name, or the custom color itself. */
export const hueOf = (color: TagColor | null | undefined) =>
  (color && (HUES.get(color) ?? (/^#[0-9a-f]{6}$/i.test(color) ? color : null))) || '#64748b'

/** The style that colors a `.tag` or `.category-value`. */
export const labelStyle = (color: TagColor | null | undefined) =>
  ({ '--tag': hueOf(color) }) as CSSProperties

/** The color the server gives a new label next to `used`: the first unused, else the least used. */
export function unusedColor(used: TagColor[]): TagColor {
  const count = (name: TagColor) => used.filter((c) => c === name).length
  return PALETTE.map(([name]) => name).reduce((best, name) =>
    count(name) < count(best) ? name : best,
  )
}

export function tagListPath(org: string, tag: { slug: string }) {
  return `/${org}?tag=${encodeURIComponent(tag.slug)}`
}
