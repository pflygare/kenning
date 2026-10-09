import { useEffect, useRef, useState } from 'react'
import type { User } from '../api/types/User'
import { useAuth } from '../auth/context'
import Avatar from './Avatar'
import styles from './UserMenu.module.css'

export default function UserMenu({ user }: { user: User }) {
  const { logout } = useAuth()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const close = (event: MouseEvent | KeyboardEvent) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !ref.current?.contains(event.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', close)
    document.addEventListener('keydown', close)
    return () => {
      document.removeEventListener('mousedown', close)
      document.removeEventListener('keydown', close)
    }
  }, [open])

  return (
    <div className={styles.menu} ref={ref}>
      <button
        className={styles.trigger}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Account"
        onClick={() => setOpen(!open)}
      >
        <Avatar name={user.name} url={user.avatar_url} />
      </button>
      {open && (
        <div className={styles.dropdown} role="menu">
          <div className={styles.who}>
            <div className={styles.name}>{user.name}</div>
            <div className="muted">{user.email}</div>
          </div>
          {/* Protected pages send you to the login page on their own. */}
          <button className={styles.item} role="menuitem" onClick={() => void logout()}>
            Sign out
          </button>
        </div>
      )}
    </div>
  )
}
