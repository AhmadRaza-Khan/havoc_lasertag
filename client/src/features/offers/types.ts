export type Offer = {
  id: string
  title: string
  description: string
  is_active: boolean
  /** First day it shows (YYYY-MM-DD, venue date), or null for no start limit. */
  starts_on: string | null
  /** Last day it shows, inclusive, or null if it never expires. */
  ends_on: string | null
  created_at: string
  updated_at: string
}

/** Body for create and update. Empty dates are sent as null. */
export type OfferInput = {
  title: string
  description: string
  is_active: boolean
  starts_on: string | null
  ends_on: string | null
}

export type OfferListParams = {
  search?: string
  page?: number
  per_page?: number
}

/** What the mobile app shows today: live offers are visible, the rest aren't. */
export type OfferStatus = 'live' | 'scheduled' | 'expired' | 'hidden'
