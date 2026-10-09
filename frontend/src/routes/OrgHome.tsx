import { Link } from 'react-router'
import { useOrg } from './useOrg'

export default function OrgHome() {
  const org = useOrg()
  return (
    <>
      <h1>{org.name}</h1>
      <p>Pages and topics arrive in the next phase.</p>
      <p>
        <Link to={`/${org.slug}/settings/members`}>Members</Link>
      </p>
    </>
  )
}
