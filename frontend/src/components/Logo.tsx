/**
 * The Kenning mark: a mirrored K back to back with a K, in dark and bright
 * blue. Colors come from `--logo-dark` and `--logo-light`, so it follows the
 * color scheme.
 */
export function LogoMark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
      <g fill="var(--logo-dark)">
        <rect x="22.5" y="10" width="8" height="44" />
        <polygon points="22.5,24.73 12.79,10 3.21,10 22.5,39.27" />
        <polygon points="22.5,24.73 3.21,54 12.79,54 22.5,39.27" />
      </g>
      <g fill="var(--logo-light)">
        <rect x="33.5" y="10" width="8" height="44" />
        <polygon points="41.5,24.73 51.21,10 60.79,10 41.5,39.27" />
        <polygon points="41.5,24.73 60.79,54 51.21,54 41.5,39.27" />
      </g>
    </svg>
  )
}

/** The mark with the name beside it. */
export function Wordmark({ size = 28 }: { size?: number }) {
  return (
    <span className="wordmark" style={{ fontSize: size * 0.68 }}>
      <LogoMark size={size} />
      Kenning
    </span>
  )
}
