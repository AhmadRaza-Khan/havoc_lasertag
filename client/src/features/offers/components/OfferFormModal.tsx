import { useState } from 'react'
import type { FormEvent } from 'react'
import { Modal } from '@/components/ui/Modal'
import { useCreateOffer, useUpdateOffer } from '../hooks'
import type { Offer } from '../types'

const MAX_TITLE = 120
const MAX_DESCRIPTION = 2000

type OfferFormModalProps = {
  open: boolean
  /** The offer being edited, or null to create a new one. */
  offer: Offer | null
  onClose: () => void
}

type FormValues = {
  title: string
  description: string
  is_active: boolean
  starts_on: string
  ends_on: string
}

function initialValues(offer: Offer | null): FormValues {
  return {
    title: offer?.title ?? '',
    description: offer?.description ?? '',
    is_active: offer?.is_active ?? true,
    starts_on: offer?.starts_on ?? '',
    ends_on: offer?.ends_on ?? '',
  }
}

export function OfferFormModal({ open, offer, onClose }: OfferFormModalProps) {
  return (
    <Modal open={open} title={offer ? 'Edit offer' : 'Add offer'} onClose={onClose}>
      {/* Keyed so the form state resets whenever a different offer (or "new") is opened */}
      {open && <OfferForm key={offer?.id ?? 'new'} offer={offer} onDone={onClose} />}
    </Modal>
  )
}

function OfferForm({ offer, onDone }: { offer: Offer | null; onDone: () => void }) {
  const [values, setValues] = useState<FormValues>(() => initialValues(offer))
  const create = useCreateOffer()
  const update = useUpdateOffer()
  const mutation = offer ? update : create

  function set<K extends keyof FormValues>(key: K, value: FormValues[K]) {
    setValues((v) => ({ ...v, [key]: value }))
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    const input = {
      ...values,
      starts_on: values.starts_on || null,
      ends_on: values.ends_on || null,
    }
    const onSuccess = () => onDone()
    if (offer) update.mutate({ id: offer.id, input }, { onSuccess })
    else create.mutate(input, { onSuccess })
  }

  return (
    <form className="form" onSubmit={handleSubmit}>
      <label>
        Title
        <input
          required
          autoFocus
          maxLength={MAX_TITLE}
          placeholder="e.g. Weekend group deal"
          value={values.title}
          onChange={(e) => set('title', e.target.value)}
        />
      </label>

      <label>
        Description
        <textarea
          required
          rows={5}
          maxLength={MAX_DESCRIPTION}
          placeholder="e.g. Book for 6 or more players on Friday or Saturday and get 20% off."
          value={values.description}
          onChange={(e) => set('description', e.target.value)}
        />
        <span className="field-hint muted">
          {values.description.length}/{MAX_DESCRIPTION} characters
        </span>
      </label>

      <div className="form-row">
        <label>
          <span>
            Start date <span className="muted">(optional)</span>
          </span>
          <input
            type="date"
            max={values.ends_on || undefined}
            value={values.starts_on}
            onChange={(e) => set('starts_on', e.target.value)}
          />
        </label>

        <label>
          <span>
            End date <span className="muted">(optional)</span>
          </span>
          <input
            type="date"
            min={values.starts_on || undefined}
            value={values.ends_on}
            onChange={(e) => set('ends_on', e.target.value)}
          />
        </label>
      </div>

      <label className="checkbox-label">
        <input
          type="checkbox"
          checked={values.is_active}
          onChange={(e) => set('is_active', e.target.checked)}
        />
        Show in the app
      </label>

      {mutation.error && <p className="error">{mutation.error.message}</p>}

      <div className="form-actions">
        <button type="button" className="btn btn-secondary" onClick={onDone} disabled={mutation.isPending}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={mutation.isPending}>
          {mutation.isPending ? 'Saving…' : offer ? 'Save changes' : 'Add offer'}
        </button>
      </div>
    </form>
  )
}
