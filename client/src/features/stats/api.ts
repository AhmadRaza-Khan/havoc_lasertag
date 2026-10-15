import { apiRequest } from '@/lib/api-client'
import type { Paginated } from '@/lib/api-client'
import type { GameRecord, Standing, StatImport } from './types'

export const statsApi = {
  /** Starts an import; it's processed in the background (poll `getImport`). */
  upload: (file: File) => {
    const body = new FormData()
    body.append('file', file)
    return apiRequest<StatImport>('/stats/imports', { method: 'POST', body })
  },
  getImport: (id: string) => apiRequest<StatImport>(`/stats/imports/${id}`),
  recentImports: () => apiRequest<StatImport[]>('/stats/imports'),
  leaderboard: () => apiRequest<Standing[]>('/stats/leaderboard'),
  playerStanding: (id: string) => apiRequest<Standing>(`/stats/players/${id}`),
  playerGames: (id: string, page: number) =>
    apiRequest<Paginated<GameRecord>>(`/stats/players/${id}/games?page=${page}&per_page=20`),
}
