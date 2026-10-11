import { useEffect, useRef, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { apiDelete, apiPost } from '../api/client'
import type { CreateTemplateRequest } from '../api/types/CreateTemplateRequest'
import type { PageDetail } from '../api/types/PageDetail'
import type { PublishRequest } from '../api/types/PublishRequest'
import type { TemplateDetail } from '../api/types/TemplateDetail'
import { useAction } from '../hooks/useAction'
import { useMediaQuery } from '../hooks/useMediaQuery'
import { copyLink, dateOnly, dateTime, pagePath, timeAgo } from './format'
import MarkdownView from './MarkdownView'
import type { Heading } from './headings'
import LinkPreviews from './LinkPreviews'
import Outline, { DOCK_QUERY } from './Outline'
import SectionLinks from './SectionLinks'
import PageMeta from './PageMeta'
import styles from './PageView.module.css'

/** Wide enough for the details beside the page as well as the outline. */
const BESIDE_QUERY = '(min-width: 88rem)'
import { usePage } from './usePage'

/** A page as readers see it, with the draft controls for editors. */
export default function PageView() {
  const { org, api, page, setPage, error } = usePage()
  const navigate = useNavigate()
  const location = useLocation()
  const action = useAction()
  const [headings, setHeadings] = useState<Heading[]>([])
  const more = useRef<HTMLDetailsElement>(null)
  // With room for the docked column, the page's buttons sit above the outline; with room for
  // another column too, the page's details sit to its left. Otherwise both stay in the page.
  const docked = useMediaQuery(DOCK_QUERY)
  const besidePage = useMediaQuery(BESIDE_QUERY)

  // The ⋯ menu closes on a click outside it and on Escape, like other menus.
  useEffect(() => {
    const close = (event: MouseEvent | KeyboardEvent) => {
      const menu = more.current
      if (!menu?.open) return
      if (
        event instanceof KeyboardEvent
          ? event.key === 'Escape'
          : !menu.contains(event.target as Node)
      )
        menu.open = false
    }
    document.addEventListener('mousedown', close)
    document.addEventListener('keydown', close)
    return () => {
      document.removeEventListener('mousedown', close)
      document.removeEventListener('keydown', close)
    }
  }, [])

  // Keep the address in step with the title (slugs change on publish).
  useEffect(() => {
    if (page && location.pathname !== pagePath(org.slug, page)) {
      navigate(pagePath(org.slug, page) + location.hash, { replace: true })
    }
  }, [page, org.slug, location.pathname, location.hash, navigate])

  if (error) {
    return <p className="alert error">{error}</p>
  }
  if (!page) {
    return <p className="status">Loading…</p>
  }

  const shown = page.published ?? page.draft!
  const here = pagePath(org.slug, page)

  const publish = () =>
    action.run(async () => {
      const body: PublishRequest = { revision_id: page.draft!.revision_id }
      setPage(await apiPost<PageDetail>(`${api}/publish`, body))
    })
  const discard = () => {
    if (
      !window.confirm('Discard the unpublished changes? Readers keep seeing the published version.')
    )
      return
    void action.run(async () => setPage(await apiDelete<PageDetail>(`${api}/draft`)))
  }
  const saveAsTemplate = () =>
    void action.run(async () => {
      const latest = page.draft ?? page.published!
      const body: CreateTemplateRequest = {
        name: latest.title,
        body_md: latest.body_md,
        topic_ids: page.topics.map((t) => t.id),
        tag_names: page.tags.map((t) => t.name),
      }
      const template = await apiPost<TemplateDetail>(`/orgs/${org.slug}/templates`, body)
      navigate(`/${org.slug}/templates/${template.id}`)
    })
  const archive = () => {
    if (!window.confirm(`Archive "${shown.title}"? It disappears from the page list.`)) return
    void action.run(async () => {
      await apiPost(`${api}/archive`)
      navigate(`/${org.slug}`, { replace: true })
    })
  }

  const actions = (
    <div className="row">
      {!page.draft && (
        <Link className="button" to={`${here}/edit`}>
          Edit
        </Link>
      )}
      <details ref={more} className={styles.more}>
        <summary className="button" aria-label="More actions">
          ⋯
        </summary>
        <div
          className={styles.menu}
          onClick={(e) => {
            // Choosing an item closes the menu.
            if ((e.target as Element).closest('a, button')) more.current!.open = false
          }}
        >
          <Link className={`button ${styles.menuLink}`} to={`${here}/history`}>
            Page history
          </Link>
          <button onClick={() => void copyLink(here)}>Copy link</button>
          <button onClick={saveAsTemplate} disabled={action.busy}>
            Save as template
          </button>
          {page.draft && page.published && (
            <button onClick={discard} disabled={action.busy}>
              Discard unpublished changes
            </button>
          )}
          <button className="danger" onClick={archive} disabled={action.busy}>
            Archive page
          </button>
        </div>
      </details>
    </div>
  )

  const changed = page.draft?.updated_at ?? page.published_at ?? page.created_at
  // About the page, then its topics, categories and tags: a box like the outline, to the left
  // of the page when there's room, else under its title.
  const details = (
    <section className={styles.details} aria-label="Page details">
      <h2 className={styles.label}>About</h2>
      <dl className={styles.facts}>
        <dt>Title</dt>
        <dd>{shown.title}</dd>
        <dt>{page.authors.length === 1 ? 'Author' : 'Authors'}</dt>
        <dd>{page.authors.join(', ')}</dd>
        <dt>Created</dt>
        <dd title={dateTime(page.created_at)}>{dateOnly(page.created_at)}</dd>
        <dt>Last changed</dt>
        <dd title={dateTime(changed)}>{timeAgo(changed)}</dd>
      </dl>
      <h2 className={styles.label}>Topics</h2>
      <PageMeta
        org={org.slug}
        api={api}
        page={page}
        onChange={setPage}
        parts={['topics']}
        className={styles.part}
      />
      {(page.categories?.length ?? 0) > 0 && (
        <>
          <h2 className={styles.label}>Categories</h2>
          <PageMeta
            org={org.slug}
            api={api}
            page={page}
            onChange={setPage}
            parts={['categories']}
            className={`${styles.part} ${styles.categoryList}`}
          />
        </>
      )}
      <h2 className={styles.label}>Tags</h2>
      <PageMeta
        org={org.slug}
        api={api}
        page={page}
        onChange={setPage}
        parts={['tags']}
        className={styles.part}
      />
    </section>
  )

  return (
    <div className={besidePage ? styles.layout : undefined}>
      {besidePage && <div className={styles.side}>{details}</div>}
      <article className={styles.page}>
        {page.draft && (
          <div className={`notice ${styles.banner}`}>
            <span>
              {page.published ? (
                <>
                  <strong>Unpublished changes</strong> by {page.draft.author_name},{' '}
                  {timeAgo(page.draft.updated_at)}. Readers still see the published version.
                </>
              ) : (
                <>
                  <strong>Draft.</strong> This page hasn't been published yet.
                </>
              )}
            </span>
            <span className="row">
              {page.published && (
                <Link className="button small" to={`${here}/compare`}>
                  Compare
                </Link>
              )}
              <Link className="button small" to={`${here}/edit`}>
                Edit draft
              </Link>
              <button
                className="small primary"
                disabled={action.busy}
                onClick={() => void publish()}
              >
                Publish
              </button>
            </span>
          </div>
        )}
        {action.error && <p className="alert error">{action.error}</p>}

        <header className={styles.header}>
          <h1>{shown.title}</h1>
          {!docked && actions}
        </header>
        {!besidePage && details}

        {shown.body_md.trim() ? (
          <LinkPreviews
            key={page.short_id}
            org={org.slug}
            follow
            onSection={(id) => {
              headings.find((h) => h.id === id)?.element.scrollIntoView({ behavior: 'smooth' })
              history.replaceState(history.state, '', `#${encodeURIComponent(id)}`)
            }}
          >
            <SectionLinks headings={headings} linkBase={here} menu>
              <MarkdownView markdown={shown.body_md} onHeadings={setHeadings} />
            </SectionLinks>
          </LinkPreviews>
        ) : (
          <p className="muted">This page is empty.</p>
        )}
      </article>
      <Outline
        key={page.short_id}
        headings={shown.body_md.trim() ? headings : []}
        linkBase={here}
        above={docked ? <div className={styles.dockActions}>{actions}</div> : undefined}
      />
    </div>
  )
}
