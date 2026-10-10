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
// get their most open value (Open or Public if there is one). Each section brings the first
// picture Wikipedia shows in it, uploaded into Kenning. Articles already in the organization
// (same title) are skipped, or with --update get a new published version.

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
    update: { type: 'boolean', default: false },
    'no-images': { type: 'boolean', default: false },
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
const WIKI_HEADERS = {
  'user-agent': 'kenning-dev-import/0.1 (https://github.com/pflygare/kenning)',
}

async function wiki(params) {
  const query = new URLSearchParams({ format: 'json', formatversion: '2', ...params })
  const res = await fetch(`${wikiApi}?${query}`, { headers: WIKI_HEADERS })
  if (!res.ok) throw new Error(`Wikipedia: ${res.status}`)
  return res.json()
}

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
  const res = await fetch(`${wikiApi}?${params}`, { headers: WIKI_HEADERS })
  if (!res.ok) throw new Error(`Wikipedia ${title}: ${res.status}`)
  const page = (await res.json()).query?.pages?.[0]
  if (!page || page.missing || !page.extract) throw new Error(`Wikipedia has no article "${title}"`)
  return { title: page.title, text: page.extract, link: articleUrl(page.title) }
}

function articleUrl(title) {
  const base = wikiApi.replace(/\/w\/api\.php$/, '')
  return `${base}/wiki/${encodeURIComponent(title.replace(/ /g, '_'))}`
}

const MAX_IMAGES = 12
const IMAGE_TYPES = new Set(['image/jpeg', 'image/png', 'image/gif', 'image/webp'])

const stripTags = (html) =>
  html
    .replace(/<[^>]+>/g, '')
    .replace(/\[edit\]/g, '')
    .replace(/&amp;/g, '&')
    .replace(/&quot;/g, '"')
    .replace(/&#0?39;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&nbsp;/g, ' ')
    .trim()

/**
 * The first picture in each section of the article as Wikipedia renders it, keyed by the
 * section's heading ('' for the lead), with a thumbnail URL. Icons and diagrams drawn as
 * SVG are left out.
 */
async function articleImages(title) {
  const html = (await wiki({ action: 'parse', page: title, prop: 'text', redirects: '1' })).parse
    ?.text
  if (!html) return new Map()
  const firstBySection = new Map()
  const seen = new Set()
  let section = ''
  const pattern = /<h([2-4])\b[^>]*>([\s\S]*?)<\/h\1>|href="\/wiki\/(File:[^"#?]+)"/g
  for (const match of html.matchAll(pattern)) {
    if (match[1]) {
      section = stripTags(match[2])
      continue
    }
    const file = decodeURIComponent(match[3]).replace(/_/g, ' ')
    if (/\.svg$/i.test(file) || seen.has(file) || SKIPPED_SECTIONS.has(section.toLowerCase()))
      continue
    seen.add(file)
    if (!firstBySection.has(section)) firstBySection.set(section, [])
    firstBySection.get(section).push(file)
  }
  // Ask for every candidate at once, then keep the first real picture per section.
  const candidates = [...firstBySection.values()].flatMap((files) => files.slice(0, 3))
  const info = new Map()
  for (let i = 0; i < candidates.length; i += 50) {
    const result = await wiki({
      action: 'query',
      titles: candidates.slice(i, i + 50).join('|'),
      prop: 'imageinfo',
      iiprop: 'url|size|mime',
      iiurlwidth: '960',
    })
    const renamed = new Map((result.query?.normalized ?? []).map((n) => [n.to, n.from]))
    for (const page of result.query?.pages ?? []) {
      const image = page.imageinfo?.[0]
      if (image) info.set(renamed.get(page.title) ?? page.title, image)
    }
  }
  const images = new Map()
  for (const [heading, files] of firstBySection) {
    const file = files.slice(0, 3).find((f) => {
      const image = info.get(f)
      return image && IMAGE_TYPES.has(image.mime) && image.width >= 200 && image.height >= 120
    })
    if (file) images.set(heading, { file, url: info.get(file).thumburl ?? info.get(file).url })
    if (images.size >= MAX_IMAGES) break
  }
  return images
}

/** Download a Wikipedia image and upload it to Kenning; returns its markdown. */
async function copyImage({ file, url }) {
  const res = await fetch(url, { headers: WIKI_HEADERS })
  if (!res.ok) throw new Error(`${file}: ${res.status}`)
  const type = res.headers.get('content-type')?.split(';')[0] ?? 'image/jpeg'
  const name = file.replace(/^File:/, '')
  const upload = await fetch(`${org}/files?name=${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: { 'content-type': type, 'x-requested-with': 'kenning', cookie },
    body: Buffer.from(await res.arrayBuffer()),
  })
  if (!upload.ok) throw new Error(`${file}: ${upload.status} ${await upload.text()}`)
  const uploaded = await upload.json()
  const alt = name.replace(/\.[a-z0-9]+$/i, '').replace(/[[\]]/g, '')
  return `![${alt}](${uploaded.url})`
}

/**
 * Plain-text extract to markdown: "== X ==" becomes "## X", each line a paragraph, and
 * `images` (markdown keyed by heading, '' for the lead) goes under its heading.
 */
export function toMarkdown(text, link, images = new Map()) {
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
  const body = []
  let leadImage = images.get('')
  for (const item of kept) {
    if (item.heading) {
      body.push(`${'#'.repeat(Math.min(item.heading, 4))} ${item.text}`)
      if (images.has(item.text)) body.push(images.get(item.text))
    } else {
      body.push(item.text)
      // The lead picture goes after the first paragraph.
      if (leadImage) body.push(leadImage)
      leadImage = undefined
    }
  }
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

  const existing = new Map((await kenning('GET', `${org}/pages`)).map((p) => [p.title, p]))

  for (const name of articles) {
    try {
      const article = await fetchArticle(name)
      const before = existing.get(article.title)
      if (before && !values.update) {
        console.log(`Skipped ${article.title} (already there; --update replaces it)`)
        continue
      }
      const images = new Map()
      if (!values['no-images']) {
        for (const [heading, image] of await articleImages(article.title)) {
          try {
            images.set(heading, await copyImage(image))
          } catch (err) {
            console.error(`  Left out an image: ${err.message}`)
          }
        }
      }
      const body_md = toMarkdown(article.text, article.link, images)
      let page
      let revision
      if (before) {
        // A new version of the same page, so links to it keep working.
        page = await kenning('GET', `${org}/pages/${before.short_id}`)
        const base = (page.draft ?? page.published).revision_id
        const saved = await kenning('PUT', `${org}/pages/${page.short_id}/draft`, {
          base_revision_id: base,
          title: article.title,
          body_md,
        })
        revision = saved.revision_id
      } else {
        page = await kenning('POST', `${org}/pages`, { title: article.title, body_md })
        await kenning('PUT', `${org}/pages/${page.short_id}/topics`, { topic_ids: [topic.id] })
        revision = page.draft.revision_id
      }
      for (const value of required) {
        await kenning('PUT', `${org}/pages/${page.short_id}/categories`, value)
      }
      await kenning('POST', `${org}/pages/${page.short_id}/publish`, { revision_id: revision })
      existing.set(article.title, page)
      const size = `${Math.round(body_md.length / 1000)}k characters, ${images.size} images`
      console.log(`${before ? 'Updated' : 'Added'} ${article.title} (${size})`)
    } catch (err) {
      console.error(`Failed ${name}: ${err.message}`)
      process.exitCode = 1
    }
  }
}

await main()
