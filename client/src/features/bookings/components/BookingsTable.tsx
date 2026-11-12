import { useState } from 'react'
import { Link, useLocation } from 'react-router'
import { Ban, Pencil, RotateCcw, Undo2 } from 'lucide-react'
import { formatDay, formatMoney, formatTimeRange } from '@/lib/format'
import type { Booking } from '../types'
import { PaymentBadge, StatusBadge } from './StatusBadge'

type BookingsTableProps = {
  bookings: Booking[]
  onCancel: (booking: Booking) => void
  /** Mark a canceled, paid booking refunded (or not). */
  onRefund: (booking: Booking) => void
}

export function BookingsTable({ bookings, onCancel, onRefund }: BookingsTableProps) {
  // The edit page returns to this list with the same tab and filters.
  const { search } = useLocation()
  // Read once when the list loads: enough to tell played bookings apart.
  const [now] = useState(Date.now)
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr>
            <th>Booking</th>
            <th>Player</th>
            <th>Game</th>
            <th>Date &amp; time</th>
            <th>People</th>
            <th>Status</th>
            <th>Price</th>
            <th className="table-actions">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {bookings.map((b) => {
            const canceled = b.status === 'canceled'
            // Bookings already played can't be changed or canceled (the server refuses too).
            const played = Date.parse(b.end_time) <= now
            return (
              <tr key={b.id} className={canceled ? 'row-muted' : undefined}>
                <td className="mono">#{b.booking_number}</td>
                <td>
                  <div className="stack-cell">
                    <span className="strong">{b.player.full_name}</span>
                    <span className="muted">@{b.player.playername}</span>
                    {b.other_players.length > 0 && (
                      <span
                        className="muted small"
                        title={b.other_players.map((o) => o.full_name).join(', ')}
                      >
                        + {b.other_players.length} other player
                        {b.other_players.length === 1 ? '' : 's'}
                      </span>
                    )}
                  </div>
                </td>
                <td className="game-cell" title={[b.game_name, ...addOnLabels(b)].join('\n')}>
                  {b.game_name}
                  {addOnLabels(b).length > 0 && (
                    <span className="muted small game-addons">+ {addOnLabels(b).join(' · ')}</span>
                  )}
                </td>
                <td>
                  <div className="stack-cell">
                    <span>{formatDay(b.start_time)}</span>
                    <span className="muted">{formatTimeRange(b.start_time, b.end_time)}</span>
                  </div>
                </td>
                <td>{b.num_people}</td>
                <td>
                  <StatusBadge status={b.status} booking={b} />
                </td>
                <td>
                  <div className="stack-cell price-cell">
                    <span>{b.price ? formatMoney(b.price.amount, b.price.currency) : '—'}</span>
                    <PaymentBadge booking={b} />
                  </div>
                </td>
                <td className="table-actions">
                  {b.refund_status ? (
                    <button
                      type="button"
                      className="icon-btn"
                      aria-label={
                        b.refund_status === 'pending'
                          ? `Mark booking ${b.booking_number} refunded`
                          : `Mark booking ${b.booking_number} not refunded`
                      }
                      title={b.refund_status === 'pending' ? 'Mark refunded' : 'Undo refunded'}
                      onClick={() => onRefund(b)}
                    >
                      {b.refund_status === 'pending' ? <RotateCcw size={16} /> : <Undo2 size={16} />}
                    </button>
                  ) : canceled || played ? (
                    <span className="icon-btn-placeholder" />
                  ) : (
                    <>
                      <Link
                        to={`/bookings/${b.id}/edit`}
                        state={{ back: `/bookings${search}` }}
                        className="icon-btn"
                        aria-label={`Edit booking ${b.booking_number}`}
                        title="Edit"
                      >
                        <Pencil size={16} />
                      </Link>
                      <button
                        type="button"
                        className="icon-btn icon-btn-danger"
                        aria-label={`Cancel booking ${b.booking_number}`}
                        title="Cancel booking"
                        onClick={() => onCancel(b)}
                      >
                        <Ban size={16} />
                      </button>
                    </>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

/** Readable add-ons of a booking: on/off ones by name when on, others as "name: value" (not 0). */
function addOnLabels(b: Booking): string[] {
  return b.options.flatMap((o) => {
    const name = (o.name ?? '').trim()
    if (o.value === 'false' || o.value.trim() === '0') return []
    if (o.value === 'true') return name ? [name] : []
    return [name ? `${name}: ${o.value.trim()}` : o.value.trim()]
  })
}
