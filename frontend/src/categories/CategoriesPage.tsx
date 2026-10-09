import { useEffect, useState } from 'react'
import { Link } from 'react-router'
import { apiDelete, apiGet, apiPatch, apiPost, errorMessage } from '../api/client'
import type { Category } from '../api/types/Category'
import type { CategoryValue } from '../api/types/CategoryValue'
import type { CreateCategoryRequest } from '../api/types/CreateCategoryRequest'
import type { CreateValueRequest } from '../api/types/CreateValueRequest'
import type { UpdateCategoryRequest } from '../api/types/UpdateCategoryRequest'
import type { UpdateValueRequest } from '../api/types/UpdateValueRequest'
import Popover from '../components/Popover'
import { useAction } from '../hooks/useAction'
import { useOrg } from '../routes/useOrg'
import ColorPicker from '../tags/ColorPicker'
import styles from './CategoriesPage.module.css'
import { valueListPath } from './paths'
import RequirementPicker, { type Requirement } from './RequirementPicker'

/** The organization's categories. Owners and admins edit them; everyone can browse by value. */
export default function CategoriesPage() {
  const org = useOrg()
  const api = `/orgs/${org.slug}/categories`
  const canManage = org.role !== 'member'
  const [categories, setCategories] = useState<Category[] | null>(null)
  const [creating, setCreating] = useState(false)
  const action = useAction()

  const { setError } = action
  useEffect(() => {
    apiGet<Category[]>(api)
      .then(setCategories)
      .catch((err: unknown) => setError(errorMessage(err)))
  }, [api, setError])

  return (
    <>
      <div className="page-header">
        <div>
          <h1>Categories</h1>
          <p>
            Fixed choices for pages, like an information class. A category can be required on every
            page or on pages in some topics; a page missing a required value can't be published.
          </p>
        </div>
        {canManage && !creating && (
          <button className="primary" onClick={() => setCreating(true)}>
            + New category
          </button>
        )}
      </div>
      {creating && (
        <NewCategoryForm
          onCreate={async (body) => {
            setCategories(await apiPost<Category[]>(api, body))
            setCreating(false)
          }}
          onCancel={() => setCreating(false)}
        />
      )}
      {action.error && <p className="alert error">{action.error}</p>}
      {categories?.length === 0 && !creating && (
        <div className="card empty">
          <h2>No categories yet</h2>
          <p>
            {canManage
              ? 'Make one with a fixed set of values, such as Information class: Open, Internal, Restricted.'
              : 'Owners and admins can add categories, such as an information class.'}
          </p>
        </div>
      )}
      {categories?.map((category) => (
        <CategoryCard
          key={category.id}
          org={org.slug}
          api={`${api}/${category.id}`}
          category={category}
          canManage={canManage}
          onChange={setCategories}
        />
      ))}
    </>
  )
}

function NewCategoryForm({
  onCreate,
  onCancel,
}: {
  onCreate: (body: CreateCategoryRequest) => Promise<void>
  onCancel: () => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [values, setValues] = useState('')
  const [requirement, setRequirement] = useState<Requirement>({ everywhere: false, topicIds: [] })
  const action = useAction()
  const valueNames = values
    .split(/[,\n]/)
    .map((v) => v.trim())
    .filter(Boolean)

  return (
    <form
      className={`card ${styles.form}`}
      onSubmit={(e) => {
        e.preventDefault()
        void action.run(() =>
          onCreate({
            name,
            description,
            values: valueNames,
            required_everywhere: requirement.everywhere,
            required_topic_ids: requirement.topicIds,
          }),
        )
      }}
    >
      <h2>New category</h2>
      <label>
        Name
        <input
          autoFocus
          maxLength={50}
          placeholder="Information class"
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
      </label>
      <label>
        Values <span className="hint">separated by commas, in the order people pick from</span>
        <input
          placeholder="Open, Internal, Restricted"
          value={values}
          onChange={(e) => setValues(e.target.value)}
        />
      </label>
      <label>
        Description <span className="hint">optional</span>
        <textarea
          rows={2}
          maxLength={500}
          placeholder="What it means and how to choose"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </label>
      <div className="field">
        Required
        <RequirementPicker label="Required" value={requirement} onChange={setRequirement} />
      </div>
      {action.error && <p className="alert error">{action.error}</p>}
      <div className="row">
        <button className="primary" disabled={action.busy || !name.trim() || valueNames.length === 0}>
          Create category
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  )
}

function CategoryCard({
  org,
  api,
  category,
  canManage,
  onChange,
}: {
  org: string
  api: string
  category: Category
  canManage: boolean
  onChange: (categories: Category[]) => void
}) {
  const action = useAction()
  /** Run a change; the server answers with the whole updated list. */
  const change = (request: () => Promise<Category[]>) =>
    action.run(async () => onChange(await request()))
  const [editing, setEditing] = useState(false)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [newValue, setNewValue] = useState('')

  const update = (body: UpdateCategoryRequest) =>
    change(() => apiPatch<Category[]>(api, body))
  const updateValue = (value: CategoryValue, body: UpdateValueRequest) =>
    change(async () => {
      const list = await apiPatch<Category[]>(`${api}/values/${value.id}`, body)
      setRenaming(null)
      return list
    })
  const addValue = () =>
    change(async () => {
      const body: CreateValueRequest = { name: newValue }
      const list = await apiPost<Category[]>(`${api}/values`, body)
      setNewValue('')
      return list
    })
  const removeValue = (value: CategoryValue) => {
    if (!window.confirm(`Delete the value "${value.name}"?`)) return
    void change(() => apiDelete<Category[]>(`${api}/values/${value.id}`))
  }
  const remove = () => {
    const used = category.values.reduce((n, v) => n + v.page_count, 0)
    const pages = used === 1 ? '1 page loses its value' : `${used} pages lose their values`
    if (!window.confirm(`Delete the category "${category.name}"? ${pages}.`)) return
    void change(() => apiDelete<Category[]>(api))
  }

  const requirement = category.required_everywhere
    ? 'Required on every page'
    : category.required_topics.length
      ? `Required in ${category.required_topics.map((t) => t.name).join(', ')} and their sub-topics`
      : 'Optional'

  return (
    <section className={`card ${styles.category}`} aria-label={category.name}>
      {editing ? (
        <EditCategoryForm
          category={category}
          onSave={(body) =>
            change(async () => {
              const list = await apiPatch<Category[]>(api, body)
              setEditing(false)
              return list
            })
          }
          onCancel={() => setEditing(false)}
        />
      ) : (
        <div className={styles.heading}>
          <div>
            <h2>{category.name}</h2>
            {category.description && <p className="card-description">{category.description}</p>}
          </div>
          {canManage && (
            <Popover label="⋯" ariaLabel={`Actions for ${category.name}`} triggerClass="small ghost">
              {(close) => (
                <div role="menu">
                  <button className="menu-item" role="menuitem" onClick={() => (close(), setEditing(true))}>
                    Rename or describe
                  </button>
                  <div className="menu-separator" />
                  <button className="menu-item danger" role="menuitem" onClick={() => (close(), remove())}>
                    Delete category
                  </button>
                </div>
              )}
            </Popover>
          )}
        </div>
      )}

      {canManage ? (
        <RequirementPicker
          label={`Where ${category.name} is required`}
          value={{
            everywhere: category.required_everywhere,
            topicIds: category.required_topics.map((t) => t.id),
          }}
          onChange={(r) =>
            void update({ required_everywhere: r.everywhere, required_topic_ids: r.topicIds })
          }
        />
      ) : (
        <p className="muted">{requirement}</p>
      )}
      {action.error && <p className="alert error">{action.error}</p>}
      {category.missing_count > 0 && (
        <p className={styles.missing}>
          {category.missing_count === 1
            ? '1 page needs a value'
            : `${category.missing_count} pages need a value`}
        </p>
      )}

      <ul className={styles.values} aria-label={`${category.name} values`}>
        {category.values.map((value) => (
          <li key={value.id} className={styles.value}>
            {canManage && (
              <ColorPicker value={value.color} onChange={(color) => void updateValue(value, { color })} />
            )}
            {renaming === value.id ? (
              <RenameForm
                name={value.name}
                onSave={(name) => void updateValue(value, { name })}
                onCancel={() => setRenaming(null)}
              />
            ) : (
              <Link to={valueListPath(org, value)} className={`tag value ${value.color}`}>
                {value.name}
              </Link>
            )}
            <span className={`muted ${styles.count}`}>
              {value.page_count === 1 ? '1 page' : `${value.page_count} pages`}
            </span>
            {canManage && (
              <Popover label="⋯" ariaLabel={`Actions for ${value.name}`} triggerClass="small ghost">
                {(close) => (
                  <div role="menu">
                    <button className="menu-item" role="menuitem" onClick={() => (close(), setRenaming(value.id))}>
                      Rename
                    </button>
                    <button
                      className="menu-item danger"
                      role="menuitem"
                      onClick={() => (close(), removeValue(value))}
                    >
                      Delete value
                    </button>
                  </div>
                )}
              </Popover>
            )}
          </li>
        ))}
      </ul>
      {canManage && (
        <form
          className={styles.addValue}
          onSubmit={(e) => {
            e.preventDefault()
            if (newValue.trim()) void addValue()
          }}
        >
          <input
            aria-label={`New value for ${category.name}`}
            placeholder="+ Add value"
            maxLength={50}
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
          />
          {newValue.trim() && <button className="small primary">Add</button>}
        </form>
      )}
    </section>
  )
}

function EditCategoryForm({
  category,
  onSave,
  onCancel,
}: {
  category: Category
  onSave: (body: UpdateCategoryRequest) => Promise<void>
  onCancel: () => void
}) {
  const [name, setName] = useState(category.name)
  const [description, setDescription] = useState(category.description)
  return (
    <form
      className={styles.form}
      onSubmit={(e) => {
        e.preventDefault()
        void onSave({ name, description })
      }}
    >
      <label>
        Name
        <input autoFocus maxLength={50} value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      <label>
        Description <span className="hint">optional</span>
        <textarea
          rows={2}
          maxLength={500}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />
      </label>
      <div className="row">
        <button className="primary" disabled={!name.trim()}>
          Save
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </form>
  )
}

function RenameForm({
  name: initial,
  onSave,
  onCancel,
}: {
  name: string
  onSave: (name: string) => void
  onCancel: () => void
}) {
  const [name, setName] = useState(initial)
  return (
    <form
      className={styles.rename}
      onSubmit={(e) => {
        e.preventDefault()
        if (name.trim() && name.trim() !== initial) onSave(name)
        else onCancel()
      }}
    >
      <input
        autoFocus
        aria-label={`Rename ${initial}`}
        maxLength={50}
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === 'Escape' && onCancel()}
      />
      <button className="small primary">Save</button>
      <button type="button" className="small" onClick={onCancel}>
        Cancel
      </button>
    </form>
  )
}
