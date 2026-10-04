import type { OfferStatus } from '../types'

const LABELS: Record<OfferStatus, { text: string; title: string }> = {
  live: { text: 'Live', title: 'Showing in the app now' },
  scheduled: { text: 'Scheduled', title: 'Shows in the app from its start date' },
  expired: { text: 'Expired', title: 'Its end date has passed' },
  hidden: { text: 'Hidden', title: 'Turned off; not shown in the app' },
}

export function OfferStatusBadge({ status }: { status: OfferStatus }) {
  const { text, title } = LABELS[status]
  return (
    <span className={`badge badge-${status}`} title={title}>
      {text}
    </span>
  )
}
