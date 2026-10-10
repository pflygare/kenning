import { Extension } from '@tiptap/core'
import type { Editor } from '@tiptap/react'
import type { Node } from '@tiptap/pm/model'
import { Plugin, type Transaction } from '@tiptap/pm/state'
import { apiUpload, errorMessage } from '../api/client'
import type { UploadedFile } from '../api/types/UploadedFile'

/** What the server accepts; it checks the bytes too. */
export const IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp']
const MAX_SIZE = 10 * 1024 * 1024

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    imageUpload: {
      /** Ask for image files and insert them at the cursor. */
      pickImage: () => ReturnType
    }
  }
}

export type ImageUploadOptions = {
  org: string
  /** Told when uploads start and finish, and why one failed. */
  onUploading: (change: 1 | -1) => void
  onError: (message: string) => void
}

const imagesIn = (files: FileList | null | undefined) =>
  [...(files ?? [])].filter((f) => f.type.startsWith('image/'))

/** "diagram-v2.png" becomes the alt text "diagram v2". */
const altText = (name: string) => name.replace(/\.[a-z0-9]+$/i, '').replace(/[-_]+/g, ' ')

/**
 * Upload images and insert them, in order, at `pos` (the cursor when absent). The place
 * is followed through edits made while the uploads run, so typing on doesn't move them.
 */
export async function insertImages(
  editor: Editor,
  options: ImageUploadOptions,
  files: File[],
  pos?: number,
) {
  let at = pos ?? blockPlace(editor.state.doc, editor.state.selection.to, true)
  const follow = ({ transaction }: { transaction: Transaction }) => {
    at = transaction.mapping.map(at)
  }
  editor.on('transaction', follow)
  try {
    for (const file of files) {
      if (!IMAGE_TYPES.includes(file.type)) {
        options.onError(`${file.name} isn't a PNG, JPEG, GIF or WebP image.`)
        continue
      }
      if (file.size > MAX_SIZE) {
        options.onError(`${file.name} is larger than 10 MB.`)
        continue
      }
      options.onUploading(1)
      try {
        const uploaded = await apiUpload<UploadedFile>(
          `/orgs/${options.org}/files`,
          file,
          file.name || 'image',
        )
        if (editor.isDestroyed) return
        const image = { type: 'image', attrs: { src: uploaded.url, alt: altText(uploaded.name) } }
        editor.chain().focus().insertContentAt(at, image).run()
        // The next image goes after this one.
        at = blockPlace(editor.state.doc, editor.state.selection.to, true)
      } catch (err) {
        options.onError(`${file.name || 'The image'} couldn't be added: ${errorMessage(err)}`)
      } finally {
        options.onUploading(-1)
      }
    }
  } finally {
    editor.off('transaction', follow)
  }
}

/**
 * Where an image inserted at `pos` goes: an empty paragraph is replaced, and at the start or
 * end of a paragraph it goes before or after it. In the middle, a paste splits the paragraph
 * like any pasted block; a drop lands after it instead.
 */
export function blockPlace(doc: Node, pos: number, split: boolean) {
  const $pos = doc.resolve(pos)
  const block = $pos.parent
  if (!block.isTextblock || $pos.depth === 0 || block.content.size === 0) return pos
  if ($pos.parentOffset === 0) return $pos.before()
  if ($pos.parentOffset === block.content.size || !split) return $pos.after()
  return pos
}

/** Images pasted, dropped or picked from a menu are uploaded and placed in the page. */
export const ImageUpload = Extension.create<ImageUploadOptions>({
  name: 'imageUpload',

  addOptions() {
    return { org: '', onUploading: () => {}, onError: () => {} }
  },

  addCommands() {
    return {
      pickImage: () => () => {
        const input = document.createElement('input')
        input.type = 'file'
        input.accept = IMAGE_TYPES.join(',')
        input.multiple = true
        input.onchange = () => void insertImages(this.editor, this.options, imagesIn(input.files))
        input.click()
        return true
      },
    }
  },

  addProseMirrorPlugins() {
    const editor = this.editor
    const options = this.options
    return [
      new Plugin({
        props: {
          handlePaste: (_view, event) => {
            const files = imagesIn(event.clipboardData?.files)
            if (!files.length) return false
            event.preventDefault()
            void insertImages(editor, options, files)
            return true
          },
          handleDrop: (view, event, _slice, moved) => {
            // Moving an image already in the page is the editor's own business.
            if (moved) return false
            const files = imagesIn(event.dataTransfer?.files)
            if (!files.length) return false
            event.preventDefault()
            const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos
            void insertImages(
              editor,
              options,
              files,
              pos === undefined ? undefined : blockPlace(view.state.doc, pos, false),
            )
            return true
          },
        },
      }),
    ]
  },
})
