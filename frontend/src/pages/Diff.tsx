import { diffLines } from 'diff'
import styles from './Diff.module.css'

type Content = { title: string; body_md: string }

/** The page as markdown, with its title as the first heading, for comparing. */
function text(content: Content) {
  return `# ${content.title}\n\n${content.body_md}`
}

/** A line diff of two versions' markdown; `same` shows when nothing differs. */
export default function Diff({
  before,
  after,
  same,
}: {
  before: Content
  after: Content
  same: string
}) {
  const parts = diffLines(text(before), text(after))
  if (!parts.some((part) => part.added || part.removed)) {
    return <p className="card">{same}</p>
  }
  return (
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
  )
}

/** "Removed" and "added" in the diff's colors, for explaining it. */
export function Removed({ children }: { children: string }) {
  return <span className={styles.removedKey}>{children}</span>
}

export function Added({ children }: { children: string }) {
  return <span className={styles.addedKey}>{children}</span>
}
