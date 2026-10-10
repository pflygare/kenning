import { Link } from 'react-router'
import type { PageSummary } from '../api/types/PageSummary'
import ChipColumn from '../components/ChipColumn'
import { valueListPath } from '../categories/paths'
import TagChip from '../tags/TagChip'
import { pagePath, timeAgo } from './format'
import styles from './PageList.module.css'

/**
 * Pages as rows, in columns: title and status, category values, tags, and who
 * changed it last. Chips that don't fit their column collapse into "+N".
 */
export default function PageRows({ org, pages }: { org: string; pages: PageSummary[] }) {
  return (
    <ul className={`card ${styles.list} ${styles.columns}`}>
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
          <ChipColumn
            className={styles.values}
            label="categories"
            chips={[
              ...page.values.map((value) => ({
                key: value.id,
                node: (
                  <Link
                    to={valueListPath(org, value)}
                    className={`category-value ${value.color}`}
                    title={`${value.category}: ${value.name}`}
                  >
                    {value.name}
                  </Link>
                ),
              })),
              ...(page.missing_required
                ? [
                    {
                      key: 'missing',
                      node: (
                        <span
                          className="category-value missing"
                          title="A required category has no value, so it can't be published"
                        >
                          Missing
                        </span>
                      ),
                    },
                  ]
                : []),
            ]}
          />
          <ChipColumn
            className={styles.tags}
            label="tags"
            chips={page.tags.map((tag) => ({
              key: tag.id,
              node: <TagChip org={org} tag={tag} />,
            }))}
          />
          <span className={`muted ${styles.meta}`}>
            {page.updated_by_name} · {timeAgo(page.updated_at)}
          </span>
        </li>
      ))}
    </ul>
  )
}
