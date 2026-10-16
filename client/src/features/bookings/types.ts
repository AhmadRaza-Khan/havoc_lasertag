export type BookingStatus = 'confirmed' | 'pending' | 'canceled'

/** Online payment (app bookings). `not_required`: nothing to pay online, e.g. staff bookings. */
export type PaymentStatus = 'not_required' | 'unpaid' | 'paid'

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
  /** Add-ons as stored in Bookeo (choice: the value's name; on/off: "true"/"false"). */
  options: BookingOption[]
  payment_status: PaymentStatus
  /** Unpaid bookings are canceled automatically after this. */
  payment_due_at: string | null
  player: { id: string; full_name: string; playername: string }
  /** Other registered players in the group (num_people includes them). */
  other_players: { id: string; full_name: string; playername: string }[]
  created_at: string
  updated_at: string
}

export type BookingOption = { id: string | null; name: string | null; value: string }

/** A chosen add-on: the game option's id and its value (choice: the value's id). */
export type OptionInput = { id: string; value: string }

export type CreateBookingInput = {
  player_id: string
  product_id: string
  event_id: string
  /** Everyone in the group: the player, the other players and any unregistered guests. */
  num_people: number
  other_player_ids?: string[]
  options?: OptionInput[]
}

export type UpdateBookingInput = {
  event_id: string
  num_people: number
  /** Replaces the other players (omit to keep them). */
  other_player_ids?: string[]
  /** Replaces the add-ons (omit to keep them). */
  options?: OptionInput[]
}

export type BookingListParams = {
  search?: string
  when?: 'upcoming' | 'past'
  status?: BookingStatus
  page?: number
  per_page?: number
}
