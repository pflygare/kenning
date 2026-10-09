import { EditorContent, useEditor } from '@tiptap/react'
import { useEffect } from 'react'
import { extensions } from './editorExtensions'

/** Read-only rendering of stored markdown, styled exactly like the editor. */
export default function MarkdownView({ markdown }: { markdown: string }) {
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

  return <EditorContent editor={editor} className="prose" />
}
