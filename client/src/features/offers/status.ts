import { BUSINESS_TIMEZONE } from '@/lib/config'
import type { Offer, OfferStatus } from './types'

// en-CA formats as YYYY-MM-DD, so dates compare as strings.
const venueDate = new Intl.DateTimeFormat('en-CA', { timeZone: BUSINESS_TIMEZONE })

/** Today's date at the venue, as YYYY-MM-DD. The server uses the same "today" for the app. */
export function venueToday(): string {
  return venueDate.format(new Date())
}

export function offerStatus(offer: Offer, today = venueToday()): OfferStatus {
  if (!offer.is_active) return 'hidden'
  if (offer.starts_on && offer.starts_on > today) return 'scheduled'
  if (offer.ends_on && offer.ends_on < today) return 'expired'
  return 'live'
}
