import { EditorContent, useEditor } from '@tiptap/react'
import { useEffect } from 'react'
import { extensions } from './editorExtensions'
import { readHeadings, type Heading } from './headings'

/** Read-only rendering of stored markdown, styled exactly like the editor. */
export default function MarkdownView({
  markdown,
  onHeadings,
}: {
  markdown: string
  /** Called with the rendered headings whenever the content changes. */
  onHeadings?: (headings: Heading[]) => void
}) {
  const editor = useEditor({
    extensions: extensions(),
    content: markdown,
    contentType: 'markdown',
    editable: false,
  })

  useEffect(() => {
    if (editor && editor.getMarkdown() !== markdown) {
      editor.commands.setContent(markdown, { contentType: 'markdown' })
    }
  }, [editor, markdown])

  useEffect(() => {
    if (!editor || !onHeadings) return
    const report = () => onHeadings(readHeadings(editor.view.dom))
    report()
    editor.on('update', report)
    return () => {
      editor.off('update', report)
    }
  }, [editor, onHeadings])

  return <EditorContent editor={editor} className="prose" />
}
