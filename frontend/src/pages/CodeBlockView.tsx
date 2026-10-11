import { NodeViewContent, NodeViewWrapper, type ReactNodeViewProps } from '@tiptap/react'
import { useMemo, useState } from 'react'
import { languages, lowlight } from './codeBlock'
import styles from './CodeBlockView.module.css'

const labelOf = (id: string | null | undefined) =>
  languages.find(([lang]) => lang === id)?.[1] ?? id ?? null

/**
 * A code block with its language in the corner: a picker while editing, a label and a copy
 * button while reading. Without a picked language the code is colored as what it looks like.
 */
export default function CodeBlockView({ node, editor, updateAttributes }: ReactNodeViewProps) {
  const picked = (node.attrs.language as string | null) || null
  const text = node.textContent
  // Guessed only for the picker, so a wrong guess is easy to correct while editing.
  const detected = useMemo(
    () =>
      picked || !editor.isEditable || !text.trim()
        ? null
        : (lowlight.highlightAuto(text).data?.language ?? null),
    [picked, editor.isEditable, text],
  )
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <NodeViewWrapper className={styles.block}>
      <div className={styles.corner} contentEditable={false}>
        {editor.isEditable ? (
          <select
            aria-label="Code language"
            value={picked ?? ''}
            onChange={(e) => updateAttributes({ language: e.target.value || null })}
          >
            <option value="">{detected ? `Auto (${labelOf(detected)})` : 'Auto-detect'}</option>
            {picked && !languages.some(([id]) => id === picked) && (
              <option value={picked}>{picked}</option>
            )}
            {languages.map(([id, label]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
          </select>
        ) : (
          <>
            {picked && <span className={styles.language}>{labelOf(picked)}</span>}
            <button
              type="button"
              className={styles.copy}
              aria-label="Copy code"
              title="Copy code"
              onClick={() => void copy()}
            >
              {copied ? 'Copied' : 'Copy'}
            </button>
          </>
        )}
      </div>
      <pre spellCheck={false}>
        <NodeViewContent<'code'> as="code" className={picked ? `language-${picked}` : undefined} />
      </pre>
    </NodeViewWrapper>
  )
}
