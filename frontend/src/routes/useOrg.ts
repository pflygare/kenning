import { useOutletContext } from 'react-router'
import type { Membership } from '../api/types/Membership'

/** The organization resolved by `OrgLayout`, for pages under `/:org`. */
export function useOrg(): Membership {
  return useOutletContext<Membership>()
}
