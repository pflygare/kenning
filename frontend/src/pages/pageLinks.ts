import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import type { EditorView } from '@tiptap/pm/view'
import { apiGet } from '../api/client'
import type { PageDetail } from '../api/types/PageDetail'
import { pagePath } from './format'
import { headingsFromMarkdown } from './headings'

export type PageLink = { shortId: string; section: string | null }

/**
 * The page (and section) a link points to, when it is a page of this organization:
 * `/acme/p/deploy-runbook-k3j9x0a2qz#rollback`, with or without this site's address.
 */
export function parsePageLink(href: string, org: string): PageLink | null {
  let url: URL
  try {
    url = new URL(href, window.location.origin)
  } catch {
    return null
  }
  if (url.origin !== window.location.origin) return null
  const match = /^\/([^/]+)\/p\/[^/]*?([a-z0-9]{10})\/?$/.exec(url.pathname)
  if (!match || decodeURIComponent(match[1]) !== org) return null
  const section = decodeURIComponent(url.hash.slice(1))
  return { shortId: match[2], section: section || null }
}

/** The address a page link stores: a path, so it keeps working on any host. */
export function pageLinkHref(
  org: string,
  page: { slug: string; short_id: string },
  section?: string | null,
): string {
  return pagePath(org, page) + (section ? `#${encodeURIComponent(section)}` : '')
}

/** What readers see of a page: its published version, or the draft before the first publish. */
export function shownVersion(page: PageDetail) {
  return page.published ?? page.draft!
}

/** "Deploy runbook › Rollback", the words a new link to a page shows. */
export function pageLinkLabel(page: PageDetail, section: string | null): string {
  const shown = shownVersion(page)
  const heading = section && headingsFromMarkdown(shown.body_md).find((h) => h.id === section)
  return heading ? `${shown.title} › ${heading.text}` : shown.title
}

const loaded = new Map<string, Promise<PageDetail>>()

/** A page by its short id, fetched once and shared by previews and pasted links. */
export function loadPage(org: string, shortId: string): Promise<PageDetail> {
  const key = `${org}/${shortId}`
  let page = loaded.get(key)
  if (!page) {
    page = apiGet<PageDetail>(`/orgs/${org}/pages/${shortId}`)
    // Forget failures, and anything older than a minute, so edits show up.
    page.catch(() => loaded.delete(key))
    window.setTimeout(() => loaded.delete(key), 60_000)
    loaded.set(key, page)
  }
  return page
}

export type PageLinksOptions = {
  org: string
  /** Shows the page picker; the editor screen draws it. */
  openPicker: () => void
}

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    pageLinks: {
      /** Find a page and link to it, or to one of its sections. */
      openPageLinkPicker: () => ReturnType
    }
  }
}

/** After a pasted page address arrives as a link, swap its text for the page's title. */
function retitle(view: EditorView, href: string, label: string) {
  const { state } = view
  const linkType = state.schema.marks.link
  const tr = state.tr
  state.doc.descendants((node, pos) => {
    if (!node.isText || node.text !== href) return
    const link = node.marks.find((m) => m.type === linkType && m.attrs.href === href)
    if (!link) return
    const at = tr.mapping.map(pos)
    tr.replaceWith(at, at + node.nodeSize, state.schema.text(label, node.marks))
  })
  if (tr.docChanged) view.dispatch(tr)
}

/**
 * Links to other pages. Pasting a page's address makes a link: over selected text it
 * links that text, otherwise it adds the page's title (and section) as the link.
 */
export const PageLinks = Extension.create<PageLinksOptions>({
  name: 'pageLinks',
  // Before the link extension, which would otherwise make the address a plain link.
  priority: 1100,

  addOptions() {
    return { org: '', openPicker: () => {} }
  },

  addCommands() {
    return {
      openPageLinkPicker: () => () => {
        this.options.openPicker()
        return true
      },
    }
  },

  addProseMirrorPlugins() {
    const org = this.options.org
    return [
      new Plugin({
        key: new PluginKey('pageLinks'),
        props: {
          handlePaste: (view, event) => {
            const text = event.clipboardData?.getData('text/plain').trim() ?? ''
            if (!text || /\s/.test(text)) return false
            const target = parsePageLink(text, org)
            const linkType = view.state.schema.marks.link
            if (!target || !linkType) return false
            const { from, to, empty, $from } = view.state.selection
            if ($from.parent.type.spec.code) return false
            const url = new URL(text, window.location.origin)
            const href = url.pathname + url.hash
            const mark = linkType.create({ href })
            if (!empty) {
              view.dispatch(view.state.tr.addMark(from, to, mark))
              return true
            }
            // Show the address as a link at once; the title replaces it when it arrives.
            // Typing on after it starts plain text again.
            view.dispatch(
              view.state.tr
                .replaceSelectionWith(view.state.schema.text(href, [mark]), false)
                .setStoredMarks([]),
            )
            loadPage(org, target.shortId)
              .then((page) => {
                if (!view.isDestroyed) retitle(view, href, pageLinkLabel(page, target.section))
              })
              .catch(() => {})
            return true
          },
        },
      }),
    ]
  },
})
