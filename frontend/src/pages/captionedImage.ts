import type { Editor } from '@tiptap/core'
import Image from '@tiptap/extension-image'

/**
 * An image block whose caption shows under it. The caption is the image's title, which
 * markdown keeps as `![alt](src "caption")`.
 */
export const CaptionedImage = Image.extend({
  parseHTML() {
    return [
      {
        tag: 'figure',
        getAttrs: (el) => {
          const img = el.querySelector('img')
          if (!img?.getAttribute('src')) return false
          return {
            src: img.getAttribute('src'),
            alt: img.getAttribute('alt'),
            title: el.querySelector('figcaption')?.textContent?.trim() || img.getAttribute('title'),
          }
        },
      },
      ...(this.parent?.() ?? []),
    ]
  },

  renderHTML({ node, HTMLAttributes }) {
    const img = ['img', { ...this.options.HTMLAttributes, ...HTMLAttributes }] as const
    const caption = node.attrs.title as string | null
    return caption ? ['figure', {}, img, ['figcaption', {}, caption]] : img
  },
})

/** Whether the selection is a single image, as after clicking or right-clicking one. */
export const imageSelected = (editor: Editor) => editor.isActive('image')

/** Ask for a new caption or alt text for the selected image. */
export function editImageText(editor: Editor, which: 'caption' | 'alt') {
  const attrs = editor.getAttributes('image')
  const current = ((which === 'caption' ? attrs.title : attrs.alt) as string | null) ?? ''
  const label =
    which === 'caption'
      ? 'Caption shown under the image (leave empty for none)'
      : 'Alt text, read aloud to people who can’t see the image'
  const answer = window.prompt(label, current)
  if (answer === null) return
  // Straight double quotes would end the caption early in the stored markdown.
  const text = answer.trim().replace(/"/g, '”') || null
  editor
    .chain()
    .focus()
    .updateAttributes('image', which === 'caption' ? { title: text } : { alt: text ?? '' })
    .run()
}
