import type { RevisionSummary } from '../api/types/RevisionSummary'
import { dateTime } from './format'

/** What happened to a revision: live, published earlier, the draft, discarded. */
export default function RevisionBadges({ revision }: { revision: RevisionSummary }) {
  return (
    <>
      {revision.is_live ? (
        <span className="badge live">Live</span>
      ) : (
        revision.published_at && (
          <span className="badge" title={`Published ${dateTime(revision.published_at)}`}>
            Published
          </span>
        )
      )}
      {revision.is_current && !revision.is_live && (
        <span className="badge draft">Current draft</span>
      )}
      {revision.discarded_at && (
        <span className="badge" title={`Discarded ${dateTime(revision.discarded_at)}`}>
          Discarded
        </span>
      )}
    </>
  )
}
