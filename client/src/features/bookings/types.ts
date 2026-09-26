export type BookingStatus = 'confirmed' | 'pending' | 'canceled'

export type Booking = {
  id: string
  booking_number: string
  product_id: string
  game_name: string
  event_id: string
  start_time: string
  end_time: string
  num_people: number
  status: BookingStatus
  price: { amount: string; currency: string } | null
  player: { id: string; full_name: string; playername: string }
  created_at: string
  updated_at: string
}

export type CreateBookingInput = {
  player_id: string
  product_id: string
  event_id: string
  num_people: number
}

export type UpdateBookingInput = {
  event_id: string
  num_people: number
}

export type BookingListParams = {
  search?: string
  when?: 'upcoming' | 'past'
  status?: BookingStatus
  page?: number
  per_page?: number
}
