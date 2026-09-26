import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { gameKeys } from '@/features/games'
import { bookingsApi } from './api'
import type { BookingListParams, CreateBookingInput, UpdateBookingInput } from './types'

export const bookingKeys = {
  all: ['bookings'] as const,
  list: (params: BookingListParams) => [...bookingKeys.all, 'list', params] as const,
  detail: (id: string) => [...bookingKeys.all, 'detail', id] as const,
}

export function useBookings(params: BookingListParams) {
  return useQuery({
    queryKey: bookingKeys.list(params),
    queryFn: () => bookingsApi.list(params),
    placeholderData: keepPreviousData,
  })
}

export function useBooking(id: string | undefined) {
  return useQuery({
    queryKey: bookingKeys.detail(id ?? ''),
    queryFn: () => bookingsApi.get(id!),
    enabled: id !== undefined,
  })
}

/** Bookings change seat counts, so slot lists are refreshed along with bookings. */
function useInvalidateAfterWrite() {
  const qc = useQueryClient()
  return () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: bookingKeys.all }),
      qc.invalidateQueries({ queryKey: gameKeys.all }),
    ])
}

export function useCreateBooking() {
  const invalidate = useInvalidateAfterWrite()
  return useMutation({
    mutationFn: (input: CreateBookingInput) => bookingsApi.create(input),
    onSuccess: invalidate,
  })
}

export function useUpdateBooking() {
  const invalidate = useInvalidateAfterWrite()
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateBookingInput }) =>
      bookingsApi.update(id, input),
    onSuccess: invalidate,
  })
}

export function useCancelBooking() {
  const invalidate = useInvalidateAfterWrite()
  return useMutation({
    mutationFn: (id: string) => bookingsApi.cancel(id),
    onSuccess: invalidate,
  })
}
