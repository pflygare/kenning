import { Markdown } from '@tiptap/markdown'
import Placeholder from '@tiptap/extension-placeholder'
import StarterKit from '@tiptap/starter-kit'
import { CaptionedImage } from './captionedImage'

/** Editor features. Everything here round-trips through markdown, which is what we store. */
export function extensions(placeholder?: string) {
  return [
    StarterKit.configure({
      // No markdown syntax for underline; keep content portable.
      underline: false,
      link: { openOnClick: false, autolink: true },
    }),
    // Images are their own blocks, with an optional caption; uploads come from ImageUpload.
    CaptionedImage.configure({ HTMLAttributes: { loading: 'lazy' } }),
    Markdown,
    ...(placeholder ? [Placeholder.configure({ placeholder })] : []),
  ]
}
