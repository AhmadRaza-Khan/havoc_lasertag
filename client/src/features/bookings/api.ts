import { apiRequest, toQueryString } from '@/lib/api-client'
import type { Paginated } from '@/lib/api-client'
import type { Booking, BookingListParams, CreateBookingInput, UpdateBookingInput } from './types'

export const bookingsApi = {
  list: (params: BookingListParams) =>
    apiRequest<Paginated<Booking>>(`/bookings${toQueryString(params)}`),
  /** Also refreshes the booking's status from Bookeo. */
  get: (id: string) => apiRequest<Booking>(`/bookings/${id}`),
  create: (input: CreateBookingInput) =>
    apiRequest<Booking>('/bookings', { method: 'POST', body: JSON.stringify(input) }),
  update: (id: string, input: UpdateBookingInput) =>
    apiRequest<Booking>(`/bookings/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
  /** Cancels in Bookeo; the booking stays listed as "canceled". */
  cancel: (id: string) => apiRequest<Booking>(`/bookings/${id}`, { method: 'DELETE' }),
  /** Records that a canceled, paid booking's money was refunded (or undoes that). */
  setRefund: (id: string, status: 'refunded' | 'pending') =>
    apiRequest<Booking>(`/bookings/${id}/refund`, { method: 'PUT', body: JSON.stringify({ status }) }),
}
