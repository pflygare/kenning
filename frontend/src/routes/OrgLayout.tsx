import { Outlet, useParams } from 'react-router'
import { useAuth } from '../auth/context'
import NotFound from './NotFound'

/** Resolves `/:org` to one of the user's organizations for the pages below it. */
export default function OrgLayout() {
  const { org: slug } = useParams()
  const { me } = useAuth()
  const org = me?.orgs.find((o) => o.slug === slug)
  if (!org) {
    return <NotFound />
  }
  return <Outlet context={org} />
}
