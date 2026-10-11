import type { Editor } from '@tiptap/react'
import { tones } from './callout'

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
      !['bulletList', 'orderedList', 'taskList', 'blockquote'].some((name) => e.isActive(name)),
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
    id: 'checklist',
    label: 'Checklist',
    icon: '☑',
    keywords: ['todo', 'tasks', 'checkbox', 'list'],
    isActive: (e) => e.isActive('taskList'),
    apply: (e) => (e.isActive('taskList') ? null : chain(e).toggleTaskList().run()),
  },
  {
    id: 'note',
    label: 'Note',
    icon: 'ℹ',
    keywords: ['callout', 'info', 'alert', 'admonition', 'box', 'panel'],
    isActive: (e) => e.isActive('callout'),
    apply: (e) => (e.isActive('callout') ? null : chain(e).setCallout('note').run()),
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

/** A note in each style, found in the "/" menu by the style's name ("/warning"). */
const noteStyles: Format[] = tones
  .filter((tone) => tone.id !== 'note')
  .map((tone) => ({
    id: tone.id,
    label: `${tone.label} note`,
    icon: tone.icon,
    keywords: ['note', 'callout', 'alert'],
    isActive: (e) => e.isActive('callout', { tone: tone.id }),
    apply: (e) => chain(e).setCallout(tone.id).run(),
  }))

/** Inserted at the cursor from the "/" menu. */
export const insertFormats: Format[] = [
  {
    id: 'table',
    label: 'Table',
    icon: '▦',
    keywords: ['grid', 'rows', 'columns', 'cells'],
    isActive: () => false,
    apply: (e) => chain(e).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  {
    id: 'divider',
    label: 'Divider',
    icon: '―',
    keywords: ['hr', 'rule', 'line', 'separator'],
    isActive: () => false,
    apply: (e) => chain(e).setHorizontalRule().run(),
  },
  {
    id: 'page',
    label: 'Link to page…',
    icon: '↗',
    keywords: ['link', 'page', 'reference', 'wiki', 'section'],
    isActive: () => false,
    apply: (e) => e.commands.openPageLinkPicker(),
  },
  {
    id: 'image',
    label: 'Image…',
    icon: '🖼',
    keywords: ['picture', 'photo', 'img', 'upload', 'screenshot'],
    isActive: () => false,
    apply: (e) => e.commands.pickImage(),
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
    id: 'highlight',
    label: 'Highlight',
    icon: '🖍',
    keywords: [],
    isActive: (e) => e.isActive('highlight'),
    apply: (e) => chain(e).toggleHighlight().run(),
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

/** Row and column changes, offered by the right-click menu inside a table. */
export const tableActions: { label: string; icon: string; run: (editor: Editor) => void }[] = [
  { label: 'Add row above', icon: '↑', run: (e) => chain(e).addRowBefore().run() },
  { label: 'Add row below', icon: '↓', run: (e) => chain(e).addRowAfter().run() },
  { label: 'Add column left', icon: '←', run: (e) => chain(e).addColumnBefore().run() },
  { label: 'Add column right', icon: '→', run: (e) => chain(e).addColumnAfter().run() },
  { label: 'Delete row', icon: '−', run: (e) => chain(e).deleteRow().run() },
  { label: 'Delete column', icon: '−', run: (e) => chain(e).deleteColumn().run() },
  { label: 'Delete table', icon: '✕', run: (e) => chain(e).deleteTable().run() },
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
  return [...all, ...noteStyles]
    .map((f) => [f, score(f)] as const)
    .filter(([, s]) => s < 3)
    .sort((a, b) => a[1] - b[1])
    .map(([f]) => f)
}
