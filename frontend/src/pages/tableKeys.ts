import { Extension, findParentNode } from '@tiptap/core'
import type { Node as PMNode } from '@tiptap/pm/model'
import { type EditorState, TextSelection, type Transaction } from '@tiptap/pm/state'

const isCell = (node: PMNode) => node.type.name === 'tableCell' || node.type.name === 'tableHeader'

/**
 * Where the cursor is in a table: the table, and the row and column it's in. Text selected
 * inside one cell counts too, as after Tab, which selects the next cell's text.
 */
function where(state: EditorState) {
  const { selection } = state
  if (!(selection instanceof TextSelection)) return null
  const table = findParentNode((node) => node.type.name === 'table')(selection)
  const cell = findParentNode(isCell)(selection)
  if (!table || !cell || selection.to > cell.pos + cell.node.nodeSize) return null
  const { $from } = state.selection
  const row = $from.index(table.depth)
  const column = $from.index(table.depth + 1)
  const rowNode = table.node.child(row)
  const last = row === table.node.childCount - 1
  // Header rows stay: removing one would leave a table markdown can't store.
  const empty = row > 0 && rowNode.textContent.trim() === '' && !hasImages(rowNode)
  const atStart = selection.empty && $from.parentOffset === 0 && $from.index(cell.depth) === 0
  return { table, row, column, last, empty, atStart }
}

const hasImages = (node: PMNode) => {
  let found = false
  node.descendants((child) => {
    found ||= !child.isText && child.isLeaf && child.type.name !== 'hardBreak'
    return !found
  })
  return found
}

/** Start of row `row`, cell `column` of the table at `tablePos`, in `doc`. */
function cellPos(doc: PMNode, tablePos: number, row: number, column: number) {
  const table = doc.nodeAt(tablePos)!
  let pos = tablePos + 1
  for (let r = 0; r < row; r++) pos += table.child(r).nodeSize
  const rowNode = table.child(row)
  pos += 1
  for (let c = 0; c < Math.min(column, rowNode.childCount - 1); c++)
    pos += rowNode.child(c).nodeSize
  return pos
}

/** Put the cursor at the end of a cell's text. */
function toCell(tr: Transaction, tablePos: number, row: number, column: number) {
  const pos = cellPos(tr.doc, tablePos, row, column)
  const cell = tr.doc.nodeAt(pos)!
  tr.setSelection(TextSelection.near(tr.doc.resolve(pos + cell.nodeSize - 1), -1))
}

/**
 * Leave the table for the line below it, as Enter on an empty bullet leaves a list. With
 * `dropRow`, the cursor's row (the empty last one) goes first.
 */
function leave(
  state: EditorState,
  dispatch: ((tr: Transaction) => void) | undefined,
  dropRow: boolean,
) {
  const at = where(state)
  if (!at) return false
  if (!dispatch) return true
  const tr = state.tr
  const { pos: tablePos, node: table } = at.table
  if (dropRow) {
    const rowStart = cellPos(state.doc, tablePos, at.row, 0) - 1
    tr.delete(rowStart, rowStart + table.child(at.row).nodeSize)
  }
  const end = tablePos + tr.doc.nodeAt(tablePos)!.nodeSize
  const next = tr.doc.nodeAt(end)
  if (!next || next.type.name !== 'paragraph') {
    tr.insert(end, state.schema.nodes.paragraph.create())
  }
  tr.setSelection(TextSelection.create(tr.doc, end + 1))
  dispatch(tr.scrollIntoView())
  return true
}

/**
 * Keys that make tables behave like lists:
 * - Enter moves to the cell below, adding a row at the bottom; Shift+Enter breaks the line.
 * - Enter or Backspace in an empty last row removes it and goes on below the table.
 * - Backspace at the start of another empty row removes it and goes to the row above.
 * - ↓ in the last row of a table at the end of the page makes a line below it.
 */
export const TableKeys = Extension.create({
  name: 'tableKeys',
  // Ahead of the table's own keys and the list and paragraph Enter handling.
  priority: 1000,

  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const at = where(editor.state)
        if (!at) return false
        if (at.last && at.empty) return leave(editor.state, editor.view.dispatch, true)
        const tablePos = at.table.pos
        if (at.last) {
          return editor
            .chain()
            .addRowAfter()
            .command(({ tr }) => {
              toCell(tr, tablePos, at.row + 1, at.column)
              return true
            })
            .run()
        }
        const tr = editor.state.tr
        toCell(tr, tablePos, at.row + 1, at.column)
        editor.view.dispatch(tr.scrollIntoView())
        return true
      },
      Backspace: ({ editor }) => {
        const at = where(editor.state)
        if (!at?.empty || !at.atStart) return false
        if (at.last) return leave(editor.state, editor.view.dispatch, true)
        const tr = editor.state.tr
        const rowStart = cellPos(tr.doc, at.table.pos, at.row, 0) - 1
        tr.delete(rowStart, rowStart + at.table.node.child(at.row).nodeSize)
        toCell(tr, at.table.pos, at.row - 1, at.column)
        editor.view.dispatch(tr.scrollIntoView())
        return true
      },
      ArrowDown: ({ editor }) => {
        const at = where(editor.state)
        if (!at?.last) return false
        const { state } = editor
        const atDocEnd = at.table.pos + at.table.node.nodeSize === state.doc.content.size
        // Only from the last line of the cell, so ↓ still moves within a multi-line cell.
        if (!atDocEnd || !editor.view.endOfTextblock('down')) return false
        return leave(state, editor.view.dispatch, false)
      },
    }
  },
})
