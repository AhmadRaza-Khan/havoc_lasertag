import type { BookingStatus } from '../types'

const LABELS: Record<BookingStatus, string> = {
  confirmed: 'Confirmed',
  pending: 'Pending',
  canceled: 'Canceled',
}

export function StatusBadge({ status }: { status: BookingStatus }) {
  return <span className={`badge badge-${status}`}>{LABELS[status]}</span>
}
