import { mergeAttributes, Node } from '@tiptap/core'
import { ReactNodeViewRenderer } from '@tiptap/react'
import CalloutView from './CalloutView'

/** The note styles, in menu order. Each is one of GitHub's alert types, so markdown stays portable. */
export const tones = [
  { id: 'note', label: 'Note', icon: 'ℹ' },
  { id: 'tip', label: 'Tip', icon: '✓' },
  { id: 'important', label: 'Important', icon: '★' },
  { id: 'warning', label: 'Warning', icon: '⚠' },
  { id: 'caution', label: 'Caution', icon: '⛔' },
] as const

export type Tone = (typeof tones)[number]['id']

const isTone = (value: unknown): value is Tone => tones.some((t) => t.id === value)

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    callout: {
      /** Wrap the selected blocks in a note, or restyle the note around them. */
      setCallout: (tone?: Tone) => ReturnType
      /** Take the note's content out of the note. */
      unsetCallout: () => ReturnType
    }
  }
}

/** First line `> [!NOTE]`, then the quoted lines that follow it. */
const CALLOUT =
  /^ {0,3}> ?\[!(note|tip|important|warning|caution)\][ \t]*\n((?: {0,3}>.*(?:\n|$))*)/i

/**
 * A colored note that holds any blocks: paragraphs, lists, headings. Stored as a GitHub alert:
 *
 *     > [!WARNING]
 *     > Text of the note
 */
export const Callout = Node.create({
  name: 'callout',
  group: 'block',
  content: 'block+',
  defining: true,

  addAttributes() {
    return {
      tone: {
        default: 'note',
        parseHTML: (el) => (isTone(el.dataset.callout) ? el.dataset.callout : 'note'),
        renderHTML: (attrs) => ({ 'data-callout': attrs.tone }),
      },
    }
  },

  parseHTML() {
    return [{ tag: 'aside[data-callout]' }]
  },

  renderHTML({ HTMLAttributes }) {
    return ['aside', mergeAttributes(HTMLAttributes), 0]
  },

  addNodeView() {
    return ReactNodeViewRenderer(CalloutView)
  },

  markdownTokenizer: {
    name: 'callout',
    level: 'block',
    start: (src: string) => src.search(/^ {0,3}> ?\[!/im),
    tokenize(src, _tokens, helpers) {
      const match = CALLOUT.exec(src)
      if (!match) return undefined
      const body = match[2].replace(/^ {0,3}> ?/gm, '')
      return {
        type: 'callout',
        raw: match[0],
        tone: match[1].toLowerCase(),
        tokens: helpers.blockTokens(body),
      }
    },
  },

  parseMarkdown: (token, helpers) => {
    const children = helpers.parseChildren(token.tokens ?? [])
    return helpers.createNode(
      'callout',
      { tone: isTone(token.tone) ? token.tone : 'note' },
      children.length ? children : [helpers.createNode('paragraph', undefined, [])],
    )
  },

  renderMarkdown: (node, h) => {
    const marker = `> [!${String(node.attrs?.tone ?? 'note').toUpperCase()}]`
    const blocks = (node.content ?? []).map((child) =>
      h
        .renderChildren([child])
        .split('\n')
        .map((line) => (line.trim() ? `> ${line}` : '>'))
        .join('\n'),
    )
    return [marker, blocks.join('\n>\n')].filter(Boolean).join('\n')
  },

  addCommands() {
    return {
      setCallout:
        (tone = 'note') =>
        ({ editor, commands }) =>
          editor.isActive('callout')
            ? commands.updateAttributes('callout', { tone })
            : commands.wrapIn('callout', { tone }),
      unsetCallout:
        () =>
        ({ commands }) =>
          commands.lift('callout'),
    }
  },
})
