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
  /** Other registered players in the group (num_people includes them). */
  other_players: { id: string; full_name: string; playername: string }[]
  created_at: string
  updated_at: string
}

export type CreateBookingInput = {
  player_id: string
  product_id: string
  event_id: string
  /** Everyone in the group: the player, the other players and any unregistered guests. */
  num_people: number
  other_player_ids?: string[]
}

export type UpdateBookingInput = {
  event_id: string
  num_people: number
  /** Replaces the other players (omit to keep them). */
  other_player_ids?: string[]
}

export type BookingListParams = {
  search?: string
  when?: 'upcoming' | 'past'
  status?: BookingStatus
  page?: number
  per_page?: number
}
