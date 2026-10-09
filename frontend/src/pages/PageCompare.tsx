import { diffLines } from 'diff'
import { Link } from 'react-router'
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

  const before = `# ${page.published.title}\n\n${page.published.body_md}`
  const after = `# ${page.draft.title}\n\n${page.draft.body_md}`
  const parts = diffLines(before, after)
  const changed = parts.some((part) => part.added || part.removed)

  return (
    <div className={styles.compare}>
      <div className="page-header">
        <div>
          <h1>Unpublished changes</h1>
          <p>
            <span className={styles.removedKey}>Removed</span> lines are live now;{' '}
            <span className={styles.addedKey}>added</span> lines go live when the draft is published.
          </p>
        </div>
        <Link className="button" to={here}>
          Back to the page
        </Link>
      </div>
      {changed ? (
        <pre className={`card ${styles.diff}`}>
          {parts.map((part, i) => (
            <span
              key={i}
              className={part.added ? styles.added : part.removed ? styles.removed : undefined}
            >
              {part.value
                .replace(/\n$/, '')
                .split('\n')
                .map((line) => `${part.added ? '+ ' : part.removed ? '- ' : '  '}${line}`)
                .join('\n') + '\n'}
            </span>
          ))}
        </pre>
      ) : (
        <p className="card">The draft matches the published version.</p>
      )}
    </div>
  )
}
