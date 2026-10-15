import { useEffect } from 'react'
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { statsApi } from './api'

export const statsKeys = {
  all: ['stats'] as const,
  leaderboard: () => [...statsKeys.all, 'leaderboard'] as const,
  imports: () => [...statsKeys.all, 'imports'] as const,
  import: (id: string) => [...statsKeys.all, 'import', id] as const,
  player: (id: string) => [...statsKeys.all, 'player', id] as const,
  playerGames: (id: string, page: number) => [...statsKeys.all, 'player', id, 'games', page] as const,
}

export function useLeaderboard() {
  return useQuery({ queryKey: statsKeys.leaderboard(), queryFn: statsApi.leaderboard })
}

export function usePlayerStanding(id: string) {
  return useQuery({ queryKey: statsKeys.player(id), queryFn: () => statsApi.playerStanding(id) })
}

export function usePlayerGames(id: string, page: number) {
  return useQuery({
    queryKey: statsKeys.playerGames(id, page),
    queryFn: () => statsApi.playerGames(id, page),
    placeholderData: keepPreviousData,
  })
}

export function useRecentImports() {
  return useQuery({ queryKey: statsKeys.imports(), queryFn: statsApi.recentImports })
}

export function useUploadStats() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (file: File) => statsApi.upload(file),
    onSuccess: (imp) => {
      qc.setQueryData(statsKeys.import(imp.id), imp)
      void qc.invalidateQueries({ queryKey: statsKeys.imports() })
    },
  })
}

/** Follows an import, polling every 2 s while it's processing. */
export function useImport(id: string | null) {
  const qc = useQueryClient()
  const query = useQuery({
    queryKey: statsKeys.import(id ?? ''),
    queryFn: () => statsApi.getImport(id!),
    enabled: id !== null,
    refetchInterval: (q) => (q.state.data?.status === 'processing' ? 2000 : false),
  })

  // When it finishes, refresh the report and the upload history.
  const status = query.data?.status
  useEffect(() => {
    if (status && status !== 'processing') {
      void qc.invalidateQueries({ queryKey: statsKeys.leaderboard() })
      void qc.invalidateQueries({ queryKey: statsKeys.imports() })
    }
  }, [status, qc])

  return query
}
