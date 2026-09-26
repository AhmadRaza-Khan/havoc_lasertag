import { apiRequest, toQueryString } from '@/lib/api-client'
import type { Game, SlotsResponse } from './types'

// Admin endpoints. The mobile app uses the same shapes under /player/games.
export const gamesApi = {
  list: () => apiRequest<Game[]>('/games'),
  slots: (productId: string, from: string, to: string) =>
    apiRequest<SlotsResponse>(
      `/games/${encodeURIComponent(productId)}/slots${toQueryString({ from, to })}`,
    ),
}
