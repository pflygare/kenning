import type { Editor } from '@tiptap/react'
import { useEffect, useState } from 'react'
import { EDIT_LINK } from './linkEditing'
import { loadPage, pageLinkLabel, parsePageLink } from './pageLinks'
import LinkForm, { type Target } from './LinkForm'

/** The link around the cursor, or the selected text to make one of. */
function targetOf(editor: Editor): Target {
  const { state } = editor
  let { from, to } = state.selection
  let href = ''
  if (editor.isActive('link')) {
    href = (editor.getAttributes('link').href as string) ?? ''
    editor.commands.extendMarkRange('link')
    ;({ from, to } = editor.state.selection)
  }
  const coords = editor.view.coordsAtPos(from)
  return {
    from,
    to,
    text: state.doc.textBetween(from, to, ' '),
    href,
    at: { x: coords.left, y: coords.bottom + 6 },
  }
}

/** A Kenning page's address, from anywhere on this site, kept as a path like other page links. */
function normalize(href: string, org: string | undefined): string {
  const trimmed = href.trim()
  if (org && parsePageLink(trimmed, org)) {
    const url = new URL(trimmed, window.location.origin)
    return url.pathname + url.hash
  }
  // "example.com" means a website, not a path on this one.
  if (/^[\w-]+(\.[\w-]+)+(\/|$)/.test(trimmed)) return `https://${trimmed}`
  return trimmed
}

/**
 * A small form for a link's text and address. Returns what the editor screen draws; the
 * editor needs the `LinkEditing` extension.
 */
export function useLinkEditor(editor: Editor | null, org?: string) {
  const [target, setTarget] = useState<Target | null>(null)

  // Heard on the document: the editor's element is only in place once it is drawn.
  useEffect(() => {
    if (!editor) return
    const open = (event: Event) => {
      if (editor.isDestroyed || event.target !== editor.view.dom) return
      event.preventDefault()
      setTarget(targetOf(editor))
    }
    document.addEventListener(EDIT_LINK, open)
    return () => document.removeEventListener(EDIT_LINK, open)
  }, [editor])

  const close = () => {
    setTarget(null)
    editor?.commands.focus()
  }

  const apply = async (text: string, href: string) => {
    if (!editor || !target) return
    const address = normalize(href, org)
    setTarget(null)
    // A page link with no words shows the page's title, like one made with the page picker.
    const page = !text && org ? parsePageLink(address, org) : null
    const title = page
      ? await loadPage(org!, page.shortId)
          .then((detail) => pageLinkLabel(detail, page.section))
          .catch(() => null)
      : null
    if (editor.isDestroyed) return
    const shown = text || title || address
    const chain = editor.chain().focus()
    if (!address) {
      // No address: keep the words, drop the link.
      chain.setTextSelection({ from: target.from, to: target.to }).unsetLink().run()
    } else if (shown === target.text) {
      // Same words: relink them in place, keeping any bold or italics.
      chain
        .setTextSelection({ from: target.from, to: target.to })
        .setLink({ href: address })
        .setTextSelection(target.to)
        .run()
    } else {
      chain
        .insertContentAt(
          { from: target.from, to: target.to },
          { type: 'text', text: shown, marks: [{ type: 'link', attrs: { href: address } }] },
        )
        // Typing on after the link starts plain text again.
        .command(({ tr }) => {
          tr.setStoredMarks([])
          return true
        })
        .run()
    }
  }

  const remove = () => {
    if (!editor || !target) return
    editor
      .chain()
      .focus()
      .setTextSelection({ from: target.from, to: target.to })
      .unsetLink()
      .setTextSelection(target.to)
      .run()
    setTarget(null)
  }

  const form =
    target && editor ? (
      <LinkForm
        key={`${target.from}-${target.to}`}
        target={target}
        onSave={(text, href) => void apply(text, href)}
        onRemove={target.href ? remove : null}
        onCancel={close}
        onFindPage={
          editor.extensionManager.extensions.some((e) => e.name === 'pageLinks')
            ? () => {
                setTarget(null)
                editor.commands.openPageLinkPicker()
              }
            : null
        }
      />
    ) : null

  return form
}
