import { X } from 'lucide-react'
import { PlayerPicker } from './PlayerPicker'
import type { PickedPlayer } from './PlayerPicker'

type OtherPlayersProps = {
  value: PickedPlayer[]
  /** The booking's own player, who can't also be an "other" player. */
  mainPlayerId: string | null
  onAdd: (player: PickedPlayer) => void
  onRemove: (player: PickedPlayer) => void
}

/** The other registered players joining the booking: removable chips + a search to add more. */
export function OtherPlayers({ value, mainPlayerId, onAdd, onRemove }: OtherPlayersProps) {
  const exclude = [...value.map((p) => p.id), ...(mainPlayerId ? [mainPlayerId] : [])]

  return (
    <div className="other-players">
      {value.length > 0 && (
        <ul className="chips" aria-label="Other players">
          {value.map((p) => (
            <li key={p.id} className="chip">
              <span>
                <span className="strong">{p.full_name}</span>{' '}
                <span className="muted">@{p.playername}</span>
              </span>
              <button
                type="button"
                className="chip-remove"
                aria-label={`Remove ${p.full_name}`}
                onClick={() => onRemove(p)}
              >
                <X size={14} />
              </button>
            </li>
          ))}
        </ul>
      )}
      <PlayerPicker
        value={null}
        onChange={(p) => p && onAdd(p)}
        exclude={exclude}
        placeholder="Add a player to the group"
      />
    </div>
  )
}
