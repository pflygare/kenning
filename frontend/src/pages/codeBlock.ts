import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight'
import { ReactNodeViewRenderer } from '@tiptap/react'
import { common, createLowlight } from 'lowlight'
import CodeBlockView from './CodeBlockView'

export const lowlight = createLowlight(common)

/** Languages offered in a code block's picker, by the name markdown stores after ```. */
export const languages: [id: string, label: string][] = [
  ['bash', 'Bash'],
  ['c', 'C'],
  ['cpp', 'C++'],
  ['csharp', 'C#'],
  ['css', 'CSS'],
  ['diff', 'Diff'],
  ['go', 'Go'],
  ['graphql', 'GraphQL'],
  ['xml', 'HTML / XML'],
  ['ini', 'INI / TOML'],
  ['java', 'Java'],
  ['javascript', 'JavaScript'],
  ['json', 'JSON'],
  ['kotlin', 'Kotlin'],
  ['markdown', 'Markdown'],
  ['php', 'PHP'],
  ['plaintext', 'Plain text'],
  ['python', 'Python'],
  ['ruby', 'Ruby'],
  ['rust', 'Rust'],
  ['scss', 'SCSS'],
  ['shell', 'Shell session'],
  ['sql', 'SQL'],
  ['swift', 'Swift'],
  ['typescript', 'TypeScript'],
  ['yaml', 'YAML'],
]

/** Code blocks colored by language: the one picked, or else the one the code looks like. */
export const CodeBlock = CodeBlockLowlight.extend({
  addNodeView() {
    return ReactNodeViewRenderer(CodeBlockView)
  },
}).configure({ lowlight })
