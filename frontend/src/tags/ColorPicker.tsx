import type { TagColor } from '../api/types/TagColor'
import Popover from '../components/Popover'
import { TAG_HUES } from './colors'
import styles from './ColorPicker.module.css'

const COLORS = Object.keys(TAG_HUES) as TagColor[]

/** A swatch button that opens the palette of tag colors. */
export default function ColorPicker({ value, onChange }: { value: TagColor; onChange: (color: TagColor) => void }) {
  return (
    <Popover
      label={<span className={styles.swatch} style={{ background: TAG_HUES[value] }} />}
      ariaLabel={`Color: ${value}`}
      triggerClass={`small ghost ${styles.swatchButton}`}
      align="left"
    >
      {(close) => (
        <div className={styles.palette} role="radiogroup" aria-label="Color">
          {COLORS.map((c) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={c === value}
              aria-label={c}
              title={c}
              className={`${styles.paletteButton} ${c === value ? styles.current : ''}`}
              onClick={() => {
                close()
                onChange(c)
              }}
            >
              <span className={styles.swatch} style={{ background: TAG_HUES[c] }} />
            </button>
          ))}
        </div>
      )}
    </Popover>
  )
}
