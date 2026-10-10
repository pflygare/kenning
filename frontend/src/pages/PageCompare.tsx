import { Link } from 'react-router'
import Diff, { Added, Removed } from './Diff'
import { pagePath } from './format'
import styles from './PageCompare.module.css'
import { usePage } from './usePage'

/** The draft against what readers see now, as a line diff of the markdown. */
export default function PageCompare() {
  const { org, page, error } = usePage()
  if (error) return <p className="alert error">{error}</p>
  if (!page) return <p className="status">Loading…</p>
  const here = pagePath(org.slug, page)

  if (!page.published || !page.draft) {
    return (
      <div className="card empty">
        <h2>Nothing to compare</h2>
        <p>
          {page.published
            ? 'There are no unpublished changes.'
            : 'This page has never been published.'}
        </p>
        <Link className="button" to={here}>
          Back to the page
        </Link>
      </div>
    )
  }

  return (
    <div className={styles.compare}>
      <div className="page-header">
        <div>
          <h1>Unpublished changes</h1>
          <p>
            <Removed>Removed</Removed> lines are live now; <Added>added</Added> lines go live when
            the draft is published.
          </p>
        </div>
        <Link className="button" to={here}>
          Back to the page
        </Link>
      </div>
      <Diff
        before={page.published}
        after={page.draft}
        same="The draft matches the published version."
      />
    </div>
  )
}
