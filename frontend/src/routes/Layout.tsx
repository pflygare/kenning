import { Link, Outlet } from 'react-router'
import styles from './Layout.module.css'

export default function Layout() {
  return (
    <div className={styles.shell}>
      <header className={styles.header}>
        <Link to="/" className={styles.brand}>
          Kenning
        </Link>
      </header>
      <main className={styles.main}>
        <Outlet />
      </main>
    </div>
  )
}
