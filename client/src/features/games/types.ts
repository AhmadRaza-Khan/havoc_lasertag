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
  /** Add-ons offered when booking (Bookeo "options"), in display order. Bookeo adds their prices. */
  options: GameOption[]
}

/** An add-on. Send chosen ones as `options: [{ id, value }]`. */
export type GameOption = {
  id: string
  /** choice: pick one of `values` (send the value's id) · number · on_off ("true"/"false") · text */
  kind: 'choice' | 'number' | 'on_off' | 'text'
  name: string
  description: string | null
  /** Not offered to players in the app. */
  staff_only: boolean
  /** Text add-ons: must be filled in. */
  required: boolean
  values: { id: string; name: string; description: string | null }[]
  min: number | null
  max: number | null
  /** Value to preselect (value id, number or "true"/"false"). */
  default: string | null
  /** Picture set in Settings → Add-on pictures (API path; use `apiUrl`). */
  image_url: string | null
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
