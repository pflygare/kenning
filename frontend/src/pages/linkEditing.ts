import { Extension } from '@tiptap/core'

/** Sent to the editor's element; `useLinkEditor` answers it by showing its form. */
export const EDIT_LINK = 'kenning:edit-link'

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    linkEditor: {
      /** Edit the link at the cursor, or link the selected text. False if no form is drawn. */
      editLink: () => ReturnType
    }
  }
}

/** Ctrl/⌘ K and the menus open the link editor that `useLinkEditor` draws. */
export const LinkEditing = Extension.create({
  name: 'linkEditor',

  addCommands() {
    return {
      editLink:
        () =>
        ({ editor }) =>
          !editor.view.dom.dispatchEvent(new Event(EDIT_LINK, { bubbles: true, cancelable: true })),
    }
  },

  addKeyboardShortcuts() {
    return { 'Mod-k': () => this.editor.commands.editLink() }
  },
})
