import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { gamesApi } from './api'

export const gameKeys = {
  all: ['games'] as const,
  slots: (productId: string, from: string, to: string) =>
    [...gameKeys.all, 'slots', productId, from, to] as const,
}

export function useGames() {
  return useQuery({
    queryKey: gameKeys.all,
    queryFn: gamesApi.list,
    // Game settings rarely change (the server caches them for 10 minutes too)
    staleTime: 5 * 60_000,
  })
}

/** Slots change as people book: keep them fresh. */
export function useSlots(productId: string | null, from: string, to: string) {
  return useQuery({
    queryKey: gameKeys.slots(productId ?? '', from, to),
    queryFn: () => gamesApi.slots(productId!, from, to),
    enabled: productId !== null,
    staleTime: 0,
    placeholderData: keepPreviousData,
  })
}
