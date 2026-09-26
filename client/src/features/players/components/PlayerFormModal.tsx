import { useState } from 'react'
import type { FormEvent } from 'react'
import { Modal } from '@/components/ui/Modal'
import { useCreatePlayer, useUpdatePlayer } from '../hooks'
import type { Player, PlayerInput } from '../types'

type PlayerFormModalProps = {
  open: boolean
  /** The player being edited, or null to create a new one. */
  player: Player | null
  onClose: () => void
}

function initialValues(player: Player | null): PlayerInput {
  return {
    full_name: player?.full_name ?? '',
    playername: player?.playername ?? '',
    phone: player?.phone ?? '',
    email: player?.email ?? '',
    password: '',
  }
}

export function PlayerFormModal({ open, player, onClose }: PlayerFormModalProps) {
  return (
    <Modal open={open} title={player ? 'Edit player' : 'Add player'} onClose={onClose}>
      {/* Keyed so the form state resets whenever a different player (or "new") is opened */}
      {open && <PlayerForm key={player?.id ?? 'new'} player={player} onDone={onClose} />}
    </Modal>
  )
}

function PlayerForm({ player, onDone }: { player: Player | null; onDone: () => void }) {
  const [values, setValues] = useState<PlayerInput>(() => initialValues(player))
  const create = useCreatePlayer()
  const update = useUpdatePlayer()
  const mutation = player ? update : create
  const isEdit = player !== null

  function set<K extends keyof PlayerInput>(key: K, value: PlayerInput[K]) {
    setValues((v) => ({ ...v, [key]: value }))
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    const onSuccess = () => onDone()
    if (player) update.mutate({ id: player.id, input: values }, { onSuccess })
    else create.mutate(values, { onSuccess })
  }

  return (
    <form className="form" onSubmit={handleSubmit}>
      <label>
        Full name
        <input
          required
          autoFocus
          value={values.full_name}
          onChange={(e) => set('full_name', e.target.value)}
        />
      </label>

      <label>
        Playername
        <input
          required
          minLength={3}
          maxLength={32}
          pattern="[A-Za-z0-9_.\-]+"
          title="3–32 characters: letters, numbers, '_', '.' or '-'"
          autoCapitalize="none"
          spellCheck={false}
          value={values.playername}
          onChange={(e) => set('playername', e.target.value)}
        />
      </label>

      <div className="form-row">
        <label>
          <span>
            Phone <span className="muted">(optional)</span>
          </span>
          <input type="tel" value={values.phone} onChange={(e) => set('phone', e.target.value)} />
        </label>

        <label>
          <span>
            Email <span className="muted">(optional)</span>
          </span>
          <input type="email" value={values.email} onChange={(e) => set('email', e.target.value)} />
        </label>
      </div>

      <label>
        {isEdit ? 'New password' : 'Password'}
        <input
          type="password"
          autoComplete="new-password"
          required={!isEdit}
          minLength={8}
          placeholder={isEdit ? 'Leave blank to keep current password' : 'At least 8 characters'}
          value={values.password}
          onChange={(e) => set('password', e.target.value)}
        />
      </label>

      {mutation.error && <p className="error">{mutation.error.message}</p>}

      <div className="form-actions">
        <button type="button" className="btn btn-secondary" onClick={onDone} disabled={mutation.isPending}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={mutation.isPending}>
          {mutation.isPending ? 'Saving…' : isEdit ? 'Save changes' : 'Add player'}
        </button>
      </div>
    </form>
  )
}
