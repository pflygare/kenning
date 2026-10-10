import { apiPost } from '../api/client'
import type { CreatePageRequest } from '../api/types/CreatePageRequest'
import type { PageDetail } from '../api/types/PageDetail'
import type { TemplateSummary } from '../api/types/TemplateSummary'

/** Today as YYYY-MM-DD where the writer is, for {{date}}. */
function localDate() {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/**
 * Starts a page from a template (or blank) and opens the editor. `topicId`
 * puts the new page in a topic.
 */
export async function startPage(
  org: string,
  template: TemplateSummary | null,
  topicId?: string,
): Promise<{ page: PageDetail; titled: boolean } | null> {
  let title = 'Untitled'
  // The text needs the title, and the template doesn't set one: ask first.
  if (template && !template.title && template.uses_title) {
    const given = window.prompt('Title for the new page')?.trim()
    if (!given) return null
    title = given
  }
  const body: CreatePageRequest = {
    title,
    body_md: '',
    template_id: template?.id,
    topic_ids: topicId ? [topicId] : undefined,
    local_date: localDate(),
  }
  const page = await apiPost<PageDetail>(`/orgs/${org}/pages`, body)
  return { page, titled: title !== 'Untitled' || !!template?.title }
}
