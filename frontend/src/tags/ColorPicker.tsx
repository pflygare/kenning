import { useState } from 'react'
import type { TagColor } from '../api/types/TagColor'
import Popover from '../components/Popover'
import { hueOf, PALETTE } from './colors'
import styles from './ColorPicker.module.css'

const HEX = /^#?([0-9a-f]{6})$/i

/** A swatch button that opens the palette, plus a custom color by picker or hex code. */
export default function ColorPicker({
  value,
  onChange,
}: {
  value: TagColor
  onChange: (color: TagColor) => void
}) {
  const custom = value.startsWith('#')
  return (
    <Popover
      label={<span className={styles.swatch} style={{ background: hueOf(value) }} />}
      ariaLabel={`Color: ${value}`}
      triggerClass={`small ghost ${styles.swatchButton}`}
      align="left"
    >
      {(close) => (
        <div className={styles.panel}>
          <div className={styles.palette} role="radiogroup" aria-label="Color">
            {PALETTE.map(([name, hue]) => (
              <button
                key={name}
                type="button"
                role="radio"
                aria-checked={name === value}
                aria-label={name}
                title={name}
                className={`${styles.paletteButton} ${name === value ? styles.current : ''}`}
                onClick={() => {
                  close()
                  onChange(name)
                }}
              >
                <span className={styles.swatch} style={{ background: hue }} />
              </button>
            ))}
          </div>
          <Custom
            initial={custom ? value : hueOf(value)}
            current={custom}
            onPick={(color) => {
              close()
              onChange(color)
            }}
          />
        </div>
      )}
    </Popover>
  )
}

/** Any color: from the system color picker, or typed as a hex code. */
function Custom({
  initial,
  current,
  onPick,
}: {
  initial: string
  current: boolean
  onPick: (color: TagColor) => void
}) {
  const [text, setText] = useState(initial)
  const match = HEX.exec(text.trim())
  const hex = match ? `#${match[1].toLowerCase()}` : null
  return (
    <form
      className={styles.custom}
      onSubmit={(e) => {
        e.preventDefault()
        if (hex) onPick(hex)
      }}
    >
      <span className={styles.customLabel}>Custom</span>
      <input
        type="color"
        aria-label="Pick a custom color"
        className={`${styles.native} ${current ? styles.current : ''}`}
        value={hex ?? initial}
        onChange={(e) => setText(e.target.value)}
      />
      <input
        aria-label="Custom color code"
        className={styles.hex}
        value={text}
        maxLength={7}
        spellCheck={false}
        onChange={(e) => setText(e.target.value)}
      />
      <button type="submit" className="small" disabled={!hex}>
        Use
      </button>
    </form>
  )
}
