import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { playersApi } from './api'
import type { PlayerInput, PlayerListParams } from './types'

export const playerKeys = {
  all: ['players'] as const,
  list: (params: PlayerListParams) => [...playerKeys.all, 'list', params] as const,
}

export function usePlayers(params: PlayerListParams) {
  return useQuery({
    queryKey: playerKeys.list(params),
    queryFn: () => playersApi.list(params),
    // Keep showing the current page while the next page / search result loads
    placeholderData: keepPreviousData,
  })
}

export function useCreatePlayer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (input: PlayerInput) => playersApi.create(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: playerKeys.all }),
  })
}

export function useUpdatePlayer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: PlayerInput }) => playersApi.update(id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: playerKeys.all }),
  })
}

export function useDeletePlayer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => playersApi.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: playerKeys.all }),
  })
}
