import { Link, Navigate } from 'react-router'
import { useAuth } from '../auth/context'

export default function Home() {
  const { me } = useAuth()

  if (!me) {
    return (
      <>
        <h1>Kenning</h1>
        <p>A wiki for your team's knowledge.</p>
        <div className="row">
          <Link className="button" to="/signup">
            Create an account
          </Link>
          <Link to="/login">Sign in</Link>
        </div>
      </>
    )
  }

  if (me.orgs.length === 1) {
    return <Navigate to={`/${me.orgs[0].slug}`} replace />
  }

  return (
    <>
      <h1>Welcome, {me.user.name}</h1>
      {me.orgs.length === 0 ? (
        <p>You are not in any organization yet. Create one, or ask a colleague to invite you.</p>
      ) : (
        <ul>
          {me.orgs.map((org) => (
            <li key={org.id}>
              <Link to={`/${org.slug}`}>{org.name}</Link>
            </li>
          ))}
        </ul>
      )}
      <Link className="button" to="/new-org">
        Create an organization
      </Link>
    </>
  )
}
