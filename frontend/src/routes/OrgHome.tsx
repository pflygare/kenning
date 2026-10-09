import { Link } from 'react-router'
import { useOrg } from './useOrg'

export default function OrgHome() {
  const org = useOrg()
  return (
    <>
      <div className="page-header">
        <div>
          <h1>{org.name}</h1>
          <p>Your team's knowledge base.</p>
        </div>
      </div>
      <div className="card empty">
        <h2>No pages yet</h2>
        <p>Pages and topics are coming next. Meanwhile, invite the people who will write them.</p>
        {org.role !== 'member' && (
          <Link className="button primary" to={`/${org.slug}/settings/members`}>
            Invite people
          </Link>
        )}
      </div>
    </>
  )
}
