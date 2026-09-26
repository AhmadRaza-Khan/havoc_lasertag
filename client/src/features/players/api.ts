import { apiRequest, toQueryString } from '@/lib/api-client'
import type { Paginated } from '@/lib/api-client'
import type { Player, PlayerInput, PlayerListParams } from './types'

export const playersApi = {
  list: (params: PlayerListParams) =>
    apiRequest<Paginated<Player>>(`/players${toQueryString(params)}`),
  get: (id: string) => apiRequest<Player>(`/players/${id}`),
  create: (input: PlayerInput) =>
    apiRequest<Player>('/players', { method: 'POST', body: JSON.stringify(input) }),
  update: (id: string, input: PlayerInput) =>
    apiRequest<Player>(`/players/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
  delete: (id: string) => apiRequest<void>(`/players/${id}`, { method: 'DELETE' }),
}
