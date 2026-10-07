import { businessDate, formatDate, formatDay, formatTime } from '@/lib/format'
import type { Booking, BookingStatus } from '../types'

const LABELS: Record<BookingStatus, string> = {
  confirmed: 'Confirmed',
  pending: 'Pending',
  canceled: 'Canceled',
}

export function StatusBadge({ status }: { status: BookingStatus }) {
  return <span className={`badge badge-${status}`}>{LABELS[status]}</span>
}

/** Online payment state of an app booking; nothing for bookings with nothing to pay online. */
export function PaymentBadge({ booking }: { booking: Booking }) {
  if (booking.payment_status === 'paid') return <span className="badge badge-paid">Paid</span>
  if (booking.payment_status !== 'unpaid') return null
  const due = booking.payment_due_at
  // A canceled booking that was never paid needs no deadline.
  if (booking.status === 'canceled') return <span className="badge badge-canceled">Unpaid</span>
  return (
    <span
      className="badge badge-unpaid"
      title={due ? `Canceled automatically if not paid by ${formatDay(due)} ${formatTime(due)}` : undefined}
    >
      Unpaid{due ? ` · due ${dueLabel(due)}` : ''}
    </span>
  )
}

/** "14:20" today, else "Oct 9, 14:20" (venue time). */
function dueLabel(iso: string) {
  if (businessDate(iso) === businessDate()) return formatTime(iso)
  const day = formatDate(`${businessDate(iso)}T12:00:00Z`).replace(/,? \d{4}$/, '')
  return `${day}, ${formatTime(iso)}`
}
