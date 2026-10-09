import { useCallback, useState } from 'react'
import { errorMessage } from '../api/client'

/** Runs an async action, tracking whether it is busy and the error it failed with. */
export function useAction() {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const run = useCallback(async (action: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await action()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setBusy(false)
    }
  }, [])

  return { busy, error, setError, run }
}
