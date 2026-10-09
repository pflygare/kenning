import type { ErrorBody } from './types/ErrorBody'

/** An error response from the API, carrying its machine-readable code. */
export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

/** GET a JSON resource under `/api`. */
export async function apiGet<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, {
    ...init,
    headers: { Accept: 'application/json', ...init?.headers },
  })
  if (!res.ok) {
    throw await toApiError(res)
  }
  return (await res.json()) as T
}

async function toApiError(res: Response): Promise<ApiError> {
  try {
    const body = (await res.json()) as ErrorBody
    return new ApiError(res.status, body.error.code, body.error.message)
  } catch {
    return new ApiError(res.status, 'unknown', `Request failed (HTTP ${res.status})`)
  }
}
