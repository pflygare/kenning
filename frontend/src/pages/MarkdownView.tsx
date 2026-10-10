import { EditorContent, useEditor } from '@tiptap/react'
import { useEffect } from 'react'
import { extensions } from './editorExtensions'
import { useHeadings, type Heading } from './headings'

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

  const headings = useHeadings(editor)
  useEffect(() => {
    onHeadings?.(headings)
  }, [headings, onHeadings])

  return <EditorContent editor={editor} className="prose" />
}
