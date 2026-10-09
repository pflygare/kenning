/**
 * The Kenning mark: a mirrored K back to back with a K, in dark and bright
 * blue. Colors come from `--logo-dark` and `--logo-light`, so it follows the
 * color scheme.
 */
export function LogoMark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
      <g fill="var(--logo-dark)">
        <rect x="28" y="10" width="4" height="44" />
        <polygon points="28,25.92 14.11,10 3.49,10 28,38.08" />
        <polygon points="28,25.92 3.49,54 14.11,54 28,38.08" />
      </g>
      <g fill="var(--logo-light)">
        <rect x="32" y="10" width="4" height="44" />
        <polygon points="36,25.92 49.89,10 60.51,10 36,38.08" />
        <polygon points="36,25.92 60.51,54 49.89,54 36,38.08" />
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
