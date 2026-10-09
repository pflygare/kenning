/** A person's picture, or their initials when they have none. */
export default function Avatar({
  name,
  url,
  size = 32,
}: {
  name: string
  url?: string | null
  size?: number
}) {
  const style = { width: size, height: size, fontSize: size * 0.4 }
  if (url) {
    return <img className="avatar" src={url} alt="" style={style} referrerPolicy="no-referrer" />
  }
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0].toUpperCase())
    .join('')
  return (
    <span className="avatar" style={style} aria-hidden="true">
      {initials || '?'}
    </span>
  )
}
