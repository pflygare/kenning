export type Heading = { id: string; level: number; text: string; element: HTMLElement }

/**
 * Headings of rendered page content, each with an id for linking to it. The id lives only
 * here: the editor owns its DOM and would redraw a heading whose attributes we touched.
 */
export function readHeadings(root: HTMLElement): Heading[] {
  const used = new Map<string, number>()
  return [...root.querySelectorAll<HTMLElement>('h1, h2, h3')]
    .filter((element) => element.textContent?.trim())
    .map((element) => {
      const text = element.textContent!.trim()
      const base =
        text
          .toLowerCase()
          .normalize('NFKD')
          .replace(/[\u0300-\u036f]/g, '')
          .replace(/[^\p{L}\p{N}]+/gu, '-')
          .replace(/^-|-$/g, '') || 'section'
      const n = (used.get(base) ?? 0) + 1
      used.set(base, n)
      const id = n === 1 ? base : `${base}-${n}`
      return { id, level: Number(element.tagName[1]), text, element }
    })
}
