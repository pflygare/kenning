const START = '\u0002'
const END = '\u0003'

/** Text from search with matched words between \u0002 and \u0003, shown highlighted. */
export default function Marked({ text }: { text: string }) {
  const parts = text.split(START)
  return (
    <>
      {parts[0]}
      {parts.slice(1).map((part, i) => {
        const [matched, rest = ''] = part.split(END)
        return (
          <span key={i}>
            <mark>{matched}</mark>
            {rest}
          </span>
        )
      })}
    </>
  )
}
