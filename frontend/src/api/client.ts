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
export function apiGet<T>(path: string, init?: RequestInit): Promise<T> {
  return request<T>('GET', path, undefined, init)
}

/** POST a JSON body under `/api`; resolves to the parsed response, or `undefined` for 204. */
export function apiPost<T = void>(path: string, body: unknown = {}): Promise<T> {
  return request<T>('POST', path, body)
}

export function apiPut<T = void>(path: string, body: unknown): Promise<T> {
  return request<T>('PUT', path, body)
}

export function apiPatch<T = void>(path: string, body: unknown): Promise<T> {
  return request<T>('PATCH', path, body)
}

export function apiDelete<T = void>(path: string): Promise<T> {
  return request<T>('DELETE', path)
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  init?: RequestInit,
): Promise<T> {
  const headers: Record<string, string> = { Accept: 'application/json' }
  if (method !== 'GET') {
    // The server rejects writes without a JSON content type (CSRF protection).
    headers['Content-Type'] = 'application/json'
  }
  const res = await fetch(`/api${path}`, {
    ...init,
    method,
    headers: { ...headers, ...init?.headers },
    body: method === 'GET' ? undefined : JSON.stringify(body ?? {}),
  })
  if (!res.ok) {
    throw await toApiError(res)
  }
  if (res.status === 204) {
    return undefined as T
  }
  return (await res.json()) as T
}

/**
 * Upload a file as the raw request body. The header stands in for the JSON
 * content type the server otherwise requires on writes (CSRF protection).
 */
export async function apiUpload<T>(path: string, file: Blob, name: string): Promise<T> {
  const res = await fetch(`/api${path}?name=${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': file.type || 'application/octet-stream',
      'X-Requested-With': 'kenning',
    },
    body: file,
  })
  if (!res.ok) throw await toApiError(res)
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

/** A message fit to show for any thrown value. */
export function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : 'Something went wrong.'
}
