export type Game = {
  product_id: string
  name: string
  description: string | null
  images: string[]
  duration_minutes: number | null
  price: { amount: string; currency: string; per_person: boolean } | null
  min_people: number | null
  max_people: number | null
  /** Bookings stay "pending" until staff accept them. */
  requires_approval: boolean
}

export type Slot = {
  /** Pass to the create/update booking endpoints. */
  event_id: string
  start_time: string
  end_time: string
  seats_available: number
}

export type SlotsResponse = {
  product_id: string
  game_name: string
  timezone: string
  from: string
  to: string
  slots: Slot[]
}
