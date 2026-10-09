import type { ReactNode } from 'react'
import styles from './Page.module.css'

/** Centered content column for pages without a sidebar. */
export default function Page({ children, narrow }: { children: ReactNode; narrow?: boolean }) {
  return <main className={narrow ? styles.narrow : styles.main}>{children}</main>
}
