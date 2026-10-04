import { Link } from 'react-router'
import { Ban, Pencil } from 'lucide-react'
import { formatDay, formatMoney, formatTimeRange } from '@/lib/format'
import type { Booking } from '../types'
import { StatusBadge } from './StatusBadge'

type BookingsTableProps = {
  bookings: Booking[]
  onCancel: (booking: Booking) => void
}

export function BookingsTable({ bookings, onCancel }: BookingsTableProps) {
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
                <td className="game-cell" title={b.game_name}>
                  {b.game_name}
                </td>
                <td>
                  <div className="stack-cell">
                    <span>{formatDay(b.start_time)}</span>
                    <span className="muted">{formatTimeRange(b.start_time, b.end_time)}</span>
                  </div>
                </td>
                <td>{b.num_people}</td>
                <td>
                  <StatusBadge status={b.status} />
                </td>
                <td>{b.price ? formatMoney(b.price.amount, b.price.currency) : '—'}</td>
                <td className="table-actions">
                  {canceled ? (
                    <span className="icon-btn-placeholder" />
                  ) : (
                    <>
                      <Link
                        to={`/bookings/${b.id}/edit`}
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
