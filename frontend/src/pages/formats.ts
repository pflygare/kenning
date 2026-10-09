import type { Editor } from '@tiptap/react'

export type Format = {
  id: string
  label: string
  /** Shown at the left of the menu item. */
  icon: string
  /** Extra words that find this format in the "/" menu. */
  keywords: string[]
  isActive: (editor: Editor) => boolean
  /** Apply to the current selection. Block formats change every block it touches. */
  apply: (editor: Editor) => void
}

const chain = (editor: Editor) => editor.chain().focus()

/** Block styles, offered by the "/" menu and the right-click menu. */
export const blockFormats: Format[] = [
  {
    id: 'text',
    label: 'Text',
    icon: '¶',
    keywords: ['paragraph', 'p', 'normal', 'plain'],
    isActive: (e) =>
      e.isActive('paragraph') &&
      !['bulletList', 'orderedList', 'blockquote'].some((name) => e.isActive(name)),
    apply: (e) => chain(e).clearNodes().run(),
  },
  ...([1, 2, 3] as const).map(
    (level): Format => ({
      id: `h${level}`,
      label: ['Heading 1', 'Heading 2', 'Heading 3'][level - 1],
      icon: `H${level}`,
      keywords: [`h${level}`, 'heading', 'title'],
      isActive: (e) => e.isActive('heading', { level }),
      apply: (e) => chain(e).setHeading({ level }).run(),
    }),
  ),
  {
    id: 'bullet',
    label: 'Bulleted list',
    icon: '•',
    keywords: ['ul', 'bullets', 'unordered', 'list'],
    isActive: (e) => e.isActive('bulletList'),
    apply: (e) => (e.isActive('bulletList') ? null : chain(e).toggleBulletList().run()),
  },
  {
    id: 'numbered',
    label: 'Numbered list',
    icon: '1.',
    keywords: ['ol', 'ordered', 'numbers', 'list'],
    isActive: (e) => e.isActive('orderedList'),
    apply: (e) => (e.isActive('orderedList') ? null : chain(e).toggleOrderedList().run()),
  },
  {
    id: 'quote',
    label: 'Quote',
    icon: '❝',
    keywords: ['blockquote', 'citation'],
    isActive: (e) => e.isActive('blockquote'),
    apply: (e) => (e.isActive('blockquote') ? null : chain(e).setBlockquote().run()),
  },
  {
    id: 'code',
    label: 'Code block',
    icon: '{ }',
    keywords: ['codeblock', 'pre', 'snippet'],
    isActive: (e) => e.isActive('codeBlock'),
    apply: (e) => chain(e).setCodeBlock().run(),
  },
]

/** Inserted at the cursor from the "/" menu. */
export const insertFormats: Format[] = [
  {
    id: 'divider',
    label: 'Divider',
    icon: '―',
    keywords: ['hr', 'rule', 'line', 'separator'],
    isActive: () => false,
    apply: (e) => chain(e).setHorizontalRule().run(),
  },
]

/** Text styles, offered by the right-click menu. */
export const textFormats: Format[] = [
  {
    id: 'bold',
    label: 'Bold',
    icon: 'B',
    keywords: [],
    isActive: (e) => e.isActive('bold'),
    apply: (e) => chain(e).toggleBold().run(),
  },
  {
    id: 'italic',
    label: 'Italic',
    icon: 'I',
    keywords: [],
    isActive: (e) => e.isActive('italic'),
    apply: (e) => chain(e).toggleItalic().run(),
  },
  {
    id: 'strike',
    label: 'Strikethrough',
    icon: 'S',
    keywords: [],
    isActive: (e) => e.isActive('strike'),
    apply: (e) => chain(e).toggleStrike().run(),
  },
  {
    id: 'inline-code',
    label: 'Inline code',
    icon: '<>',
    keywords: [],
    isActive: (e) => e.isActive('code'),
    apply: (e) => chain(e).toggleCode().run(),
  },
  {
    id: 'link',
    label: 'Link…',
    icon: '🔗',
    keywords: [],
    isActive: (e) => e.isActive('link'),
    apply: (e) => editLink(e),
  },
]

export function editLink(editor: Editor) {
  const previous = editor.getAttributes('link').href as string | undefined
  const url = window.prompt('Link address', previous ?? 'https://')
  if (url === null) return
  if (url === '') chain(editor).extendMarkRange('link').unsetLink().run()
  else chain(editor).extendMarkRange('link').setLink({ href: url }).run()
}

/** Formats for the "/" menu whose name or keywords start with `query`, best first. */
export function matchFormats(query: string): Format[] {
  const q = query.toLowerCase()
  const all = [...blockFormats, ...insertFormats]
  if (!q) return all
  const score = (f: Format) => {
    const words = [f.id, f.label.toLowerCase(), ...f.keywords]
    if (words.some((w) => w === q)) return 0
    if (words.some((w) => w.startsWith(q))) return 1
    if (f.label.toLowerCase().includes(q)) return 2
    return 3
  }
  return all
    .map((f) => [f, score(f)] as const)
    .filter(([, s]) => s < 3)
    .sort((a, b) => a[1] - b[1])
    .map(([f]) => f)
}
