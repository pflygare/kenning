import { Link } from 'react-router'
import type { PageSummary } from '../api/types/PageSummary'
import TagChip from '../tags/TagChip'
import { pagePath, timeAgo } from './format'
import styles from './PageList.module.css'

/** Pages as rows: title, status, tags, and who changed it last. */
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
          </Link>
          {page.tags.length > 0 && (
            <span className={`chips ${styles.tags}`}>
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
