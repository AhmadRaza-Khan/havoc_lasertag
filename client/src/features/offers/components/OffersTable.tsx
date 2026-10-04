import { Pencil, Trash2 } from 'lucide-react'
import { formatDate } from '@/lib/format'
import { offerStatus, venueToday } from '../status'
import type { Offer } from '../types'
import { OfferStatusBadge } from './OfferStatusBadge'

type OffersTableProps = {
  offers: Offer[]
  onEdit: (offer: Offer) => void
  onDelete: (offer: Offer) => void
}

// Plain dates (no time): format at noon UTC so no timezone shifts them to another day.
const formatPlainDate = (d: string) => formatDate(`${d}T12:00:00Z`)

function runs(offer: Offer) {
  const { starts_on: from, ends_on: to } = offer
  if (from && to) return `${formatPlainDate(from)} – ${formatPlainDate(to)}`
  if (from) return `From ${formatPlainDate(from)}`
  if (to) return `Until ${formatPlainDate(to)}`
  return <span className="muted">Always</span>
}

export function OffersTable({ offers, onEdit, onDelete }: OffersTableProps) {
  const today = venueToday()
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr>
            <th>Offer</th>
            <th>Runs</th>
            <th>Status</th>
            <th className="table-actions">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {offers.map((o) => (
            <tr key={o.id}>
              <td>
                <div className="offer-cell">
                  <span className="offer-title">{o.title}</span>
                  <span className="offer-description muted">{o.description}</span>
                </div>
              </td>
              <td>{runs(o)}</td>
              <td>
                <OfferStatusBadge status={offerStatus(o, today)} />
              </td>
              <td className="table-actions">
                <button
                  type="button"
                  className="icon-btn"
                  aria-label={`Edit ${o.title}`}
                  title="Edit"
                  onClick={() => onEdit(o)}
                >
                  <Pencil size={16} />
                </button>
                <button
                  type="button"
                  className="icon-btn icon-btn-danger"
                  aria-label={`Delete ${o.title}`}
                  title="Delete"
                  onClick={() => onDelete(o)}
                >
                  <Trash2 size={16} />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
