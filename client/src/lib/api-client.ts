const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080'

export class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

type QueryValue = string | number | boolean | null | undefined

let onUnauthorized: (() => void) | null = null

/** Called whenever an API request returns 401 (e.g. the session expired). */
export function setUnauthorizedHandler(handler: (() => void) | null) {
  onUnauthorized = handler
}

/** Builds `?a=1&b=2`, skipping empty values. */
export function toQueryString(params: Record<string, QueryValue>): string {
  const qs = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') qs.set(key, String(value))
  }
  const s = qs.toString()
  return s ? `?${s}` : ''
}

/** Shared fetch wrapper for the Rust API. Feature `api.ts` files build on this. */
export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const res = await fetch(`${API_URL}/api${path}`, {
    ...init,
    // Send/receive the httpOnly auth cookie across the api subdomain
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...init.headers,
    },
  })

  if (!res.ok) {
    if (res.status === 401) onUnauthorized?.()
    const body = await res.json().catch(() => null)
    throw new ApiError(res.status, body?.error ?? `Request failed (${res.status})`)
  }

  if (res.status === 204) return undefined as T
  return res.json() as Promise<T>
}

/** Paginated list response shape shared by all list endpoints. */
export type Paginated<T> = {
  items: T[]
  total: number
  page: number
  per_page: number
}
