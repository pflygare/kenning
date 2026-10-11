import type { Editor } from '@tiptap/react'
import { useEffect, useState } from 'react'

export type Heading = { id: string; level: number; text: string; element: HTMLElement }

/**
 * Headings of rendered page content, each with an id for linking to it. The id lives only
 * here: the editor owns its DOM and would redraw a heading whose attributes we touched.
 */
export function readHeadings(root: HTMLElement): Heading[] {
  const ids = headingIds()
  return [...root.querySelectorAll<HTMLElement>('h1, h2, h3')]
    .filter((element) => element.textContent?.trim())
    .map((element) => {
      const text = element.textContent!.trim()
      return { id: ids(text), level: Number(element.tagName[1]), text, element }
    })
}

/**
 * Gives each heading text its id, in page order: "set-up" for "Set up", then "set-up-2" for a second
 * heading with the same words.
 */
function headingIds() {
  const used = new Map<string, number>()
  return (text: string) => {
    const base =
      text
        .toLowerCase()
        .normalize('NFKD')
        .replace(/[\u0300-\u036f]/g, '')
        .replace(/[^\p{L}\p{N}]+/gu, '-')
        .replace(/^-|-$/g, '') || 'section'
    const n = (used.get(base) ?? 0) + 1
    used.set(base, n)
    return n === 1 ? base : `${base}-${n}`
  }
}

/** Markdown inline syntax dropped, leaving the words a heading shows. */
function plainText(markdown: string): string {
  return markdown
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\\(.)|[*_~`]/g, (_, escaped: string | undefined) => escaped ?? '')
    .trim()
}

/**
 * The headings of stored markdown, with the same ids the rendered page gives them, so a
 * link to a section can be made without opening the page.
 */
export function headingsFromMarkdown(markdown: string): Omit<Heading, 'element'>[] {
  const ids = headingIds()
  const found: Omit<Heading, 'element'>[] = []
  let fence: string | null = null
  for (const line of markdown.split('\n')) {
    const marker = /^\s{0,3}(`{3,}|~{3,})/.exec(line)?.[1]
    if (marker) {
      if (!fence) fence = marker[0]
      else if (marker[0] === fence) fence = null
      continue
    }
    if (fence) continue
    const match = /^\s{0,3}(#{1,3})\s+(.*?)(?:\s+#+)?\s*$/.exec(line)
    const text = match && plainText(match[2])
    if (match && text) found.push({ id: ids(text), level: match[1].length, text })
  }
  return found
}

const same = (a: Heading[], b: Heading[]) =>
  a.length === b.length &&
  a.every((h, i) => h.id === b[i].id && h.level === b[i].level && h.element === b[i].element)

/** The headings an editor (or read-only view) shows, kept current as its content changes. */
export function useHeadings(editor: Editor | null): Heading[] {
  const [headings, setHeadings] = useState<Heading[]>([])
  useEffect(() => {
    if (!editor) return
    const report = () => {
      const next = readHeadings(editor.view.dom)
      setHeadings((prev) => (same(prev, next) ? prev : next))
    }
    const first = requestAnimationFrame(report)
    editor.on('update', report)
    return () => {
      cancelAnimationFrame(first)
      editor.off('update', report)
    }
  }, [editor])
  return headings
}
