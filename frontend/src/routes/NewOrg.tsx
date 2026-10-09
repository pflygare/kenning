import { type FormEvent, useState } from 'react'
import { useNavigate } from 'react-router'
import { apiPost } from '../api/client'
import type { CreateOrgRequest } from '../api/types/CreateOrgRequest'
import type { Membership } from '../api/types/Membership'
import { useAuth } from '../auth/context'
import { useAction } from '../hooks/useAction'
import ConfirmEmail from './ConfirmEmail'

export default function NewOrg() {
  const { me, refresh } = useAuth()
  const navigate = useNavigate()
  const [name, setName] = useState('')
  const [slug, setSlug] = useState('')
  const { busy, error, run } = useAction()

  if (!me?.user.email_verified) {
    return <ConfirmEmail />
  }

  const submit = (event: FormEvent) => {
    event.preventDefault()
    void run(async () => {
      const body: CreateOrgRequest = { name, slug: slug.trim() || undefined }
      const org = await apiPost<Membership>('/orgs', body)
      await refresh()
      navigate(`/${org.slug}`)
    })
  }

  return (
    <>
      <h1>Create an organization</h1>
      <p>An organization holds your team's pages. You can invite people once it exists.</p>
      <form className="form" onSubmit={submit}>
        <label>
          Name
          <input required maxLength={100} value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label>
          Web address <span className="hint">Optional. Lowercase letters, digits and dashes.</span>
          <input
            placeholder="made from the name"
            pattern="[a-z0-9]+(-[a-z0-9]+)*"
            maxLength={48}
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
          />
        </label>
        {error && <p className="error">{error}</p>}
        <button className="primary" type="submit" disabled={busy}>
          Create organization
        </button>
      </form>
    </>
  )
}
