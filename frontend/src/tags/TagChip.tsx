import { Link } from 'react-router'
import type { TagRef } from '../api/types/TagRef'
import { labelStyle, tagListPath } from './colors'

/** A colored tag; links to the pages carrying it, or shows a remove button. */
export default function TagChip({
  org,
  tag,
  onRemove,
}: {
  org: string
  tag: TagRef
  onRemove?: () => void
}) {
  if (onRemove) {
    return (
      <span className="tag" style={labelStyle(tag.color)}>
        {tag.name}
        <button type="button" aria-label={`Remove tag ${tag.name}`} onClick={onRemove}>
          ×
        </button>
      </span>
    )
  }
  return (
    <Link className="tag" style={labelStyle(tag.color)} to={tagListPath(org, tag)}>
      {tag.name}
    </Link>
  )
}
