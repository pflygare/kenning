#!/usr/bin/env node
// Imports Wikipedia articles into a Kenning organization as published pages, for realistic
// test content: long text with section headings. Run against a dev instance:
//
//   node scripts/import-wikipedia.mjs --email you@example.com --org acme
//
// It asks for the password, or takes it from --password or KENNING_PASSWORD.
//
// Options: --url (default http://localhost:3000), --lang (default en), --topic (default
// Wikipedia; created if missing), and article titles as extra arguments. Required categories
// get their most open value (Open or Public if there is one). Articles already
// in the organization (same title) are skipped, so running it again is safe.

import { createInterface } from 'node:readline/promises'
import { parseArgs } from 'node:util'

const DEFAULT_ARTICLES = [
  'Kenning',
  'Norse mythology',
  'Wiki',
  'PostgreSQL',
  'Rust (programming language)',
  'Stockholm',
  'Coffee',
  'Aurora',
]

// Sections that are only lists of links or citations once the markup is gone.
const SKIPPED_SECTIONS = new Set([
  'see also',
  'references',
  'notes',
  'external links',
  'further reading',
  'sources',
  'bibliography',
  'citations',
  'footnotes',
  'notes and references',
])

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    url: { type: 'string', default: 'http://localhost:3000' },
    email: { type: 'string' },
    password: { type: 'string' },
    org: { type: 'string' },
    lang: { type: 'string', default: 'en' },
    topic: { type: 'string', default: 'Wikipedia' },
    'wiki-api': { type: 'string' },
  },
})
if (!values.email || !values.org) {
  console.error('Usage: import-wikipedia.mjs --email … --org <slug> [--password …] [titles…]')
  process.exit(2)
}
values.password ??= process.env.KENNING_PASSWORD ?? (await askPassword())

async function askPassword() {
  const rl = createInterface({ input: process.stdin, output: process.stdout })
  const answer = await rl.question(`Password for ${values.email}: `)
  rl.close()
  return answer
}
const articles = positionals.length ? positionals : DEFAULT_ARTICLES
const wikiApi = values['wiki-api'] ?? `https://${values.lang}.wikipedia.org/w/api.php`
const api = `${values.url.replace(/\/$/, '')}/api`
const org = `${api}/orgs/${values.org}`

let cookie = ''
async function kenning(method, url, body) {
  const res = await fetch(url, {
    method,
    headers: { 'content-type': 'application/json', cookie },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const setCookie = res.headers.getSetCookie?.() ?? []
  if (setCookie.length) cookie = setCookie.map((c) => c.split(';')[0]).join('; ')
  const text = await res.text()
  if (!res.ok) throw new Error(`${method} ${url}: ${res.status} ${text}`)
  return text ? JSON.parse(text) : null
}

/** The article as plain text with "== Section ==" markers, and its canonical title. */
async function fetchArticle(title) {
  const params = new URLSearchParams({
    action: 'query',
    prop: 'extracts',
    explaintext: '1',
    exsectionformat: 'wiki',
    redirects: '1',
    titles: title,
    format: 'json',
    formatversion: '2',
  })
  const res = await fetch(`${wikiApi}?${params}`, {
    headers: { 'user-agent': 'kenning-dev-import/0.1 (https://github.com/pflygare/kenning)' },
  })
  if (!res.ok) throw new Error(`Wikipedia ${title}: ${res.status}`)
  const page = (await res.json()).query?.pages?.[0]
  if (!page || page.missing || !page.extract) throw new Error(`Wikipedia has no article "${title}"`)
  return { title: page.title, text: page.extract, link: articleUrl(page.title) }
}

function articleUrl(title) {
  const base = wikiApi.replace(/\/w\/api\.php$/, '')
  return `${base}/wiki/${encodeURIComponent(title.replace(/ /g, '_'))}`
}

/** Plain-text extract to markdown: "== X ==" becomes "## X", each line a paragraph. */
export function toMarkdown(text, link) {
  const out = []
  let skipDepth = 0
  for (const raw of text.split('\n')) {
    const line = raw.trim()
    if (!line) continue
    const heading = /^(={2,6})\s*(.*?)\s*\1$/.exec(line)
    if (heading) {
      const depth = heading[1].length
      if (skipDepth && depth > skipDepth) continue
      skipDepth = SKIPPED_SECTIONS.has(heading[2].toLowerCase()) ? depth : 0
      if (!skipDepth) out.push({ heading: depth, text: heading[2] })
      continue
    }
    if (skipDepth) continue
    // Keep text from turning into markdown structure by accident.
    out.push({ text: line.replace(/^([#>*+-]|\d+[.)])(\s)/, '\\$1$2') })
  }
  // Drop headings left with nothing under them (their sections were math or tables).
  const kept = out.filter((item, i) => {
    if (!item.heading) return true
    const next = out.slice(i + 1).find((n) => !n.heading || n.heading <= item.heading)
    return next && !next.heading
  })
  const body = kept.map((item) =>
    item.heading ? `${'#'.repeat(Math.min(item.heading, 4))} ${item.text}` : item.text,
  )
  body.push(`*From Wikipedia: [${link}](${link}), under CC BY-SA 4.0.*`)
  return body.join('\n\n') + '\n'
}

async function main() {
  await kenning('POST', `${api}/auth/login`, { email: values.email, password: values.password })

  const topics = await kenning('GET', `${org}/topics`)
  let topic = topics.find((t) => t.name.toLowerCase() === values.topic.toLowerCase())
  if (!topic) {
    topic = (
      await kenning('POST', `${org}/topics`, {
        name: values.topic,
        description: 'Articles imported from Wikipedia as test content.',
        parent_id: null,
      })
    ).topic
    console.log(`Created topic ${topic.name}`)
  }

  // Categories some pages must have before publishing: pick the most open value.
  const required = (await kenning('GET', `${org}/categories`))
    .filter((c) => c.values.length && (c.required_everywhere || c.required_topics.length))
    .map((c) => ({
      category_id: c.id,
      value_id: (c.values.find((v) => /^(open|public)/i.test(v.name)) ?? c.values[0]).id,
    }))

  const titles = new Set((await kenning('GET', `${org}/pages`)).map((p) => p.title))

  for (const name of articles) {
    try {
      const article = await fetchArticle(name)
      if (titles.has(article.title)) {
        console.log(`Skipped ${article.title} (already there)`)
        continue
      }
      const body_md = toMarkdown(article.text, article.link)
      const page = await kenning('POST', `${org}/pages`, { title: article.title, body_md })
      await kenning('PUT', `${org}/pages/${page.short_id}/topics`, { topic_ids: [topic.id] })
      for (const value of required) {
        await kenning('PUT', `${org}/pages/${page.short_id}/categories`, value)
      }
      await kenning('POST', `${org}/pages/${page.short_id}/publish`, {
        revision_id: page.draft.revision_id,
      })
      titles.add(article.title)
      console.log(`Added ${article.title} (${Math.round(body_md.length / 1000)}k characters)`)
    } catch (err) {
      console.error(`Failed ${name}: ${err.message}`)
      process.exitCode = 1
    }
  }
}

await main()
