import { businessDate, formatDate, formatDay, formatTime } from '@/lib/format'
import type { Booking, BookingStatus } from '../types'

const LABELS: Record<BookingStatus, string> = {
  confirmed: 'Confirmed',
  pending: 'Pending',
  canceled: 'Canceled',
}

/**
 * Bookeo's status. A pending booking that has been paid says so, so staff can tell it from an
 * unpaid one waiting for approval.
 */
export function StatusBadge({ status, booking }: { status: BookingStatus; booking?: Booking }) {
  const paid = booking && (booking.payment_status === 'paid' || booking.payment_status === 'partially_paid')
  if (status === 'pending' && paid) {
    return (
      <span className="badge badge-pending" title="Paid in the app; waiting for staff approval in Bookeo">
        Paid – awaiting approval
      </span>
    )
  }
  return <span className={`badge badge-${status}`}>{LABELS[status]}</span>
}

/** Online payment state of an app booking; nothing for bookings with nothing to pay online. */
export function PaymentBadge({ booking }: { booking: Booking }) {
  // Paid, then canceled: the money is owed back until staff refund it.
  if (booking.refund_status === 'pending') return <span className="badge badge-unpaid">Refund pending</span>
  if (booking.refund_status === 'refunded') return <span className="badge badge-canceled">Refunded</span>
  if (booking.payment_status === 'paid') return <span className="badge badge-paid">Paid</span>
  if (booking.payment_status === 'partially_paid') {
    const price = booking.price
    const rest = price ? Math.max(Number(price.amount) - Number(booking.amount_paid), 0) : null
    return (
      <span
        className="badge badge-partial"
        title={
          rest !== null && price
            ? `${price.currency} ${rest.toFixed(2)} still to pay (in the app until the game starts, then at the venue)`
            : undefined
        }
      >
        Paid {Number(booking.amount_paid).toFixed(2)}
        {price ? ` of ${Number(price.amount).toFixed(2)}` : ''}
      </span>
    )
  }
  if (booking.payment_status !== 'unpaid') return null
  const due = booking.payment_due_at
  // A canceled booking that was never paid needs no deadline.
  if (booking.status === 'canceled') return <span className="badge badge-canceled">Unpaid</span>
  return (
    <span
      className="badge badge-unpaid"
      title={
        due
          ? `Canceled automatically if not paid by ${formatDay(due)} ${formatTime(due)}`
          : undefined
      }
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
