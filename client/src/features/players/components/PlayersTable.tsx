import { Pencil, Trash2 } from 'lucide-react'
import { formatDate } from '@/lib/format'
import type { Player } from '../types'

type PlayersTableProps = {
  players: Player[]
  onEdit: (player: Player) => void
  onDelete: (player: Player) => void
}

export function PlayersTable({ players, onEdit, onDelete }: PlayersTableProps) {
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr>
            <th>Player</th>
            <th>Phone</th>
            <th>Email</th>
            <th>Joined</th>
            <th className="table-actions">
              <span className="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {players.map((p) => (
            <tr key={p.id}>
              <td>
                <div className="player-cell">
                  <span className="player-name">{p.full_name}</span>
                  <span className="muted">@{p.playername}</span>
                </div>
              </td>
              <td>{p.phone ?? <span className="muted">—</span>}</td>
              <td>{p.email ?? <span className="muted">—</span>}</td>
              <td>{formatDate(p.created_at)}</td>
              <td className="table-actions">
                <button
                  type="button"
                  className="icon-btn"
                  aria-label={`Edit ${p.playername}`}
                  title="Edit"
                  onClick={() => onEdit(p)}
                >
                  <Pencil size={16} />
                </button>
                <button
                  type="button"
                  className="icon-btn icon-btn-danger"
                  aria-label={`Delete ${p.playername}`}
                  title="Delete"
                  onClick={() => onDelete(p)}
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
