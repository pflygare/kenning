import Highlight from '@tiptap/extension-highlight'
import { TaskItem, TaskList } from '@tiptap/extension-list'
import Placeholder from '@tiptap/extension-placeholder'
import { renderTableToMarkdown, Table, TableKit } from '@tiptap/extension-table'
import { Markdown } from '@tiptap/markdown'
import StarterKit from '@tiptap/starter-kit'
import { Callout } from './callout'
import { CaptionedImage } from './captionedImage'
import { CodeBlock } from './codeBlock'
import { LinkEditing } from './linkEditing'
import { TableKeys } from './tableKeys'

// The stock table adds blank lines around itself, which pile up between blocks.
const CompactTable = Table.extend({
  renderMarkdown: (node, h) => renderTableToMarkdown(node, h).trim(),
})

/** Editor features. Everything here round-trips through markdown, which is what we store. */
export function extensions(placeholder?: string) {
  return [
    StarterKit.configure({
      // No markdown syntax for underline; keep content portable.
      underline: false,
      link: { openOnClick: false, autolink: true },
      codeBlock: false,
    }),
    // Images are their own blocks, with an optional caption; uploads come from ImageUpload.
    CaptionedImage.configure({ HTMLAttributes: { loading: 'lazy' } }),
    CodeBlock,
    Callout,
    // Markdown tables have no column widths, so columns size to their content.
    TableKit.configure({ table: false }),
    CompactTable.configure({ resizable: false }),
    TableKeys,
    TaskList,
    TaskItem.configure({ nested: true }),
    Highlight,
    LinkEditing,
    Markdown,
    ...(placeholder ? [Placeholder.configure({ placeholder })] : []),
  ]
}
