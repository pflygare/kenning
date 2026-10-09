import { Link, Outlet } from 'react-router'
import { Wordmark } from '../components/Logo'
import styles from './AuthLayout.module.css'

/** Sign-in, sign-up and email-link pages: a centered card under the logo. */
export default function AuthLayout() {
  return (
    <div className={styles.page}>
      <Link to="/" className={styles.logo} aria-label="Kenning home">
        <Wordmark size={36} />
      </Link>
      <main className={styles.card}>
        <Outlet />
      </main>
    </div>
  )
}
