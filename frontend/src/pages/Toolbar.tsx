import { type Editor, useEditorState } from '@tiptap/react'
import { editLink } from './formats'
import styles from './Toolbar.module.css'

/**
 * Formatting buttons. Markdown shortcuts (`## `, `- `, `**bold**`), "/" and right-click work
 * too, and images can also be pasted or dropped in.
 */
export default function Toolbar({ editor }: { editor: Editor }) {
  const state = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive('bold'),
      italic: editor.isActive('italic'),
      code: editor.isActive('code'),
      h2: editor.isActive('heading', { level: 2 }),
      h3: editor.isActive('heading', { level: 3 }),
      bullet: editor.isActive('bulletList'),
      ordered: editor.isActive('orderedList'),
      quote: editor.isActive('blockquote'),
      codeBlock: editor.isActive('codeBlock'),
      link: editor.isActive('link'),
    }),
  })

  const chain = () => editor.chain().focus()

  const buttons: [string, string, boolean, () => void][] = [
    ['B', 'Bold', state.bold, () => chain().toggleBold().run()],
    ['I', 'Italic', state.italic, () => chain().toggleItalic().run()],
    ['H2', 'Heading', state.h2, () => chain().toggleHeading({ level: 2 }).run()],
    ['H3', 'Subheading', state.h3, () => chain().toggleHeading({ level: 3 }).run()],
    ['•', 'Bulleted list', state.bullet, () => chain().toggleBulletList().run()],
    ['1.', 'Numbered list', state.ordered, () => chain().toggleOrderedList().run()],
    ['❝', 'Quote', state.quote, () => chain().toggleBlockquote().run()],
    ['<>', 'Inline code', state.code, () => chain().toggleCode().run()],
    ['{ }', 'Code block', state.codeBlock, () => chain().toggleCodeBlock().run()],
    ['🔗', 'Link', state.link, () => editLink(editor)],
    ['↗', 'Link to page', false, () => editor.commands.openPageLinkPicker()],
    ['🖼', 'Image', false, () => editor.commands.pickImage()],
  ]

  return (
    <div className={styles.toolbar} role="toolbar" aria-label="Formatting">
      {buttons.map(([label, title, active, run]) => (
        <button
          key={title}
          type="button"
          title={title}
          aria-label={title}
          aria-pressed={active}
          className={active ? styles.active : undefined}
          onMouseDown={(e) => e.preventDefault()}
          onClick={run}
        >
          {label}
        </button>
      ))}
    </div>
  )
}
