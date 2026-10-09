import { Link } from 'react-router'
import type { PageSummary } from '../api/types/PageSummary'
import { valueListPath } from '../categories/paths'
import TagChip from '../tags/TagChip'
import { pagePath, timeAgo } from './format'
import styles from './PageList.module.css'

/** Pages as rows: title, status, category values, tags, and who changed it last. */
export default function PageRows({ org, pages }: { org: string; pages: PageSummary[] }) {
  return (
    <ul className={`card ${styles.list}`}>
      {pages.map((page) => (
        <li key={page.short_id} className={styles.row}>
          <Link to={pagePath(org, page)} className={styles.item}>
            <span className={styles.title}>{page.title}</span>
            {!page.published ? (
              <span className="badge draft">Draft</span>
            ) : (
              page.has_draft && <span className="badge draft">Unpublished changes</span>
            )}
            {page.missing_required && (
              <span className="badge draft" title="A required category has no value, so it can't be published">
                Needs a category
              </span>
            )}
          </Link>
          {(page.values.length > 0 || page.tags.length > 0) && (
            <span className={`chips ${styles.tags}`}>
              {page.values.map((value) => (
                <Link
                  key={value.id}
                  to={valueListPath(org, value)}
                  className={`tag value ${value.color}`}
                  title={`${value.category}: ${value.name}`}
                >
                  {value.name}
                </Link>
              ))}
              {page.tags.map((tag) => (
                <TagChip key={tag.id} org={org} tag={tag} />
              ))}
            </span>
          )}
          <span className={`muted ${styles.meta}`}>
            {page.updated_by_name} · {timeAgo(page.updated_at)}
          </span>
        </li>
      ))}
    </ul>
  )
}
