import { type FormEvent, useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { apiDelete, apiGet, apiPatch, apiPost, errorMessage } from '../api/client'
import type { ChangeRoleRequest } from '../api/types/ChangeRoleRequest'
import type { InviteRequest } from '../api/types/InviteRequest'
import type { Member } from '../api/types/Member'
import type { PendingInvite } from '../api/types/PendingInvite'
import type { Role } from '../api/types/Role'
import { useAuth } from '../auth/context'
import Avatar from '../components/Avatar'
import { useAction } from '../hooks/useAction'
import { useOrg } from './useOrg'

const ROLES: Role[] = ['member', 'admin', 'owner']
const ROLE_LABELS: Record<Role, string> = { member: 'Member', admin: 'Admin', owner: 'Owner' }

export default function Members() {
  const org = useOrg()
  const { me, refresh } = useAuth()
  const navigate = useNavigate()
  const [members, setMembers] = useState<Member[] | null>(null)
  const [invites, setInvites] = useState<PendingInvite[]>([])
  const [loadError, setLoadError] = useState<string | null>(null)
  const action = useAction()
  const manager = org.role !== 'member'
  const base = `/orgs/${org.slug}`

  const fetchAll = useCallback(
    () =>
      Promise.all([
        apiGet<Member[]>(`${base}/members`),
        manager ? apiGet<PendingInvite[]>(`${base}/invites`) : Promise.resolve([]),
      ]),
    [base, manager],
  )
  const load = useCallback(async () => {
    const [members, invites] = await fetchAll()
    setMembers(members)
    setInvites(invites)
  }, [fetchAll])

  useEffect(() => {
    fetchAll()
      .then(([members, invites]) => {
        setMembers(members)
        setInvites(invites)
      })
      .catch((err: unknown) => setLoadError(errorMessage(err)))
  }, [fetchAll])

  // Mirrors the server's rules, which have the final say.
  const canChange = (from: Role, to: Role) =>
    manager && (org.role === 'owner' || (from !== 'owner' && to !== 'owner'))
  const canRemove = (target: Role, isSelf: boolean) =>
    isSelf || (manager && (org.role === 'owner' || target !== 'owner'))

  const changeRole = (member: Member, role: Role) =>
    action.run(async () => {
      const body: ChangeRoleRequest = { role }
      await apiPatch(`${base}/members/${member.user_id}`, body)
      if (member.user_id === me?.user.id) await refresh()
      await load()
    })

  const remove = (member: Member) => {
    const isSelf = member.user_id === me?.user.id
    const question = isSelf
      ? `Leave ${org.name}? You will need a new invitation to come back.`
      : `Remove ${member.name} from ${org.name}?`
    if (!window.confirm(question)) return
    void action.run(async () => {
      await apiDelete(`${base}/members/${member.user_id}`)
      if (isSelf) {
        await refresh()
        navigate('/', { replace: true })
      } else {
        await load()
      }
    })
  }

  const revoke = (invite: PendingInvite) =>
    action.run(async () => {
      await apiDelete(`${base}/invites/${invite.id}`)
      await load()
    })

  if (loadError) {
    return <p className="error">{loadError}</p>
  }
  if (!members) {
    return <p className="status">Loading members…</p>
  }

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Members</h1>
          <p>
            {members.length} {members.length === 1 ? 'person' : 'people'} in {org.name}.
          </p>
        </div>
      </div>
      {manager && <InviteForm orgSlug={org.slug} ownerRole={org.role === 'owner'} onSent={load} />}
      {action.error && <p className="alert error">{action.error}</p>}
      <section className="card">
      <table className="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Role</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {members.map((member) => {
            const isSelf = member.user_id === me?.user.id
            return (
              <tr key={member.user_id}>
                <td>
                  <div className="person">
                    <Avatar name={member.name} url={member.avatar_url} />
                    <div>
                      <div className="person-name">
                        {member.name} {isSelf && <span className="badge">You</span>}
                      </div>
                      <div className="muted">{member.email}</div>
                    </div>
                  </div>
                </td>
                <td>
                  {canChange(member.role, member.role) ? (
                    <select
                      aria-label={`Role for ${member.name}`}
                      value={member.role}
                      disabled={action.busy}
                      onChange={(e) => void changeRole(member, e.target.value as Role)}
                    >
                      {ROLES.filter((r) => r === member.role || canChange(member.role, r)).map(
                        (r) => (
                          <option key={r} value={r}>
                            {ROLE_LABELS[r]}
                          </option>
                        ),
                      )}
                    </select>
                  ) : (
                    ROLE_LABELS[member.role]
                  )}
                </td>
                <td className="actions">
                  {canRemove(member.role, isSelf) && (
                    <button
                      className="link danger"
                      disabled={action.busy}
                      onClick={() => remove(member)}
                    >
                      {isSelf ? 'Leave' : 'Remove'}
                    </button>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
      </section>

      {manager && invites.length > 0 && (
        <section className="card">
          <h2>Pending invitations</h2>
          <p className="card-description">
            These people have been invited but haven't joined yet.
          </p>
          <table className="table">
            <thead>
              <tr>
                <th>Email</th>
                <th>Role</th>
                <th>Expires</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {invites.map((invite) => (
                <tr key={invite.id}>
                  <td>
                    <div className="person-name">{invite.email}</div>
                    <div className="muted">Invited by {invite.invited_by_name}</div>
                  </td>
                  <td>{ROLE_LABELS[invite.role]}</td>
                  <td>{new Date(invite.expires_at).toLocaleDateString()}</td>
                  <td className="actions">
                    <button
                      className="link danger"
                      disabled={action.busy}
                      onClick={() => void revoke(invite)}
                    >
                      Revoke
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </>
  )
}

function InviteForm({
  orgSlug,
  ownerRole,
  onSent,
}: {
  orgSlug: string
  ownerRole: boolean
  onSent: () => Promise<void>
}) {
  const [email, setEmail] = useState('')
  const [role, setRole] = useState<Role>('member')
  const [sentTo, setSentTo] = useState<string | null>(null)
  const { busy, error, run } = useAction()

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: InviteRequest = { email, role }
      await apiPost(`/orgs/${orgSlug}/invites`, body)
      setSentTo(email)
      setEmail('')
      await onSent()
    })
  }

  return (
    <form className="card" onSubmit={submit}>
      <h2>Invite people</h2>
      <p className="card-description">
        They'll get an email with a link to join, whether or not they have an account yet.
      </p>
      <div className="row">
        <input
          type="email"
          placeholder="name@example.com"
          aria-label="Email to invite"
          required
          style={{ flex: '1 1 16rem' }}
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
        <select aria-label="Role" value={role} onChange={(e) => setRole(e.target.value as Role)}>
          {ROLES.filter((r) => ownerRole || r !== 'owner').map((r) => (
            <option key={r} value={r}>
              {ROLE_LABELS[r]}
            </option>
          ))}
        </select>
        <button className="primary" type="submit" disabled={busy}>
          Send invite
        </button>
      </div>
      {error && <p className="error" style={{ marginTop: '0.75rem' }}>{error}</p>}
      {sentTo && !error && (
        <p className="muted" style={{ margin: '0.75rem 0 0', color: 'var(--success)' }}>
          Invitation sent to {sentTo}.
        </p>
      )}
    </form>
  )
}
