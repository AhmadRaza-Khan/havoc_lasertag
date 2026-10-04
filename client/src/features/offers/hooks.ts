import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { offersApi } from './api'
import type { OfferInput, OfferListParams } from './types'

export const offerKeys = {
  all: ['offers'] as const,
  list: (params: OfferListParams) => [...offerKeys.all, 'list', params] as const,
}

export function useOffers(params: OfferListParams) {
  return useQuery({
    queryKey: offerKeys.list(params),
    queryFn: () => offersApi.list(params),
    placeholderData: keepPreviousData,
  })
}

export function useCreateOffer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (input: OfferInput) => offersApi.create(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: offerKeys.all }),
  })
}

export function useUpdateOffer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: OfferInput }) => offersApi.update(id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: offerKeys.all }),
  })
}

export function useDeleteOffer() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => offersApi.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: offerKeys.all }),
  })
}
