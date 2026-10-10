import Image from '@tiptap/extension-image'
import { Markdown } from '@tiptap/markdown'
import Placeholder from '@tiptap/extension-placeholder'
import StarterKit from '@tiptap/starter-kit'

/** Editor features. Everything here round-trips through markdown, which is what we store. */
export function extensions(placeholder?: string) {
  return [
    StarterKit.configure({
      // No markdown syntax for underline; keep content portable.
      underline: false,
      link: { openOnClick: false, autolink: true },
    }),
    // Images are their own blocks; uploads come from ImageUpload in the editor.
    Image.configure({ HTMLAttributes: { loading: 'lazy' } }),
    Markdown,
    ...(placeholder ? [Placeholder.configure({ placeholder })] : []),
  ]
}
