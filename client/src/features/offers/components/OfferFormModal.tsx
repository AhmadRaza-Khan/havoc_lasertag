import { useEffect, useRef, useState } from 'react'
import type { ChangeEvent, FormEvent } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { ImagePlus, Trash2 } from 'lucide-react'
import { Modal } from '@/components/ui/Modal'
import { apiUrl } from '@/lib/api-client'
import { offersApi } from '../api'
import { offerKeys } from '../hooks'
import { prepareImage } from '../image'
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

/** The image as the form holds it: unchanged, a new one picked, or removed. */
type ImageState = { kind: 'keep' } | { kind: 'new'; blob: Blob; preview: string } | { kind: 'remove' }

function OfferForm({ offer, onDone }: { offer: Offer | null; onDone: () => void }) {
  const [values, setValues] = useState<FormValues>(() => initialValues(offer))
  const [image, setImage] = useState<ImageState>({ kind: 'keep' })
  const [preparing, setPreparing] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const fileInput = useRef<HTMLInputElement>(null)
  const qc = useQueryClient()

  // Free the preview's object URL when it's replaced or the form closes.
  const preview = image.kind === 'new' ? image.preview : null
  useEffect(() => () => void (preview && URL.revokeObjectURL(preview)), [preview])

  const shownImage = image.kind === 'new' ? image.preview : image.kind === 'keep' && offer?.image_url ? apiUrl(offer.image_url) : null
  const hasText = values.title.trim() !== '' || values.description.trim() !== ''

  function set<K extends keyof FormValues>(key: K, value: FormValues[K]) {
    setValues((v) => ({ ...v, [key]: value }))
  }

  async function pick(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (!file) return
    setError(null)
    setPreparing(true)
    try {
      const blob = await prepareImage(file)
      setImage({ kind: 'new', blob, preview: URL.createObjectURL(blob) })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not use this image.')
    } finally {
      setPreparing(false)
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!hasText && !shownImage) {
      setError('Add a title, a description or an image (or any mix of them).')
      return
    }
    const input = {
      ...values,
      title: values.title.trim(),
      description: values.description.trim(),
      starts_on: values.starts_on || null,
      ends_on: values.ends_on || null,
    }
    setSaving(true)
    setError(null)
    try {
      if (offer) {
        // A new image goes first, so the text may be cleared in the same save; removing the
        // image comes last, once the text is saved.
        if (image.kind === 'new') await offersApi.uploadImage(offer.id, image.blob)
        await offersApi.update(offer.id, input)
        if (image.kind === 'remove' && offer.image_url) await offersApi.removeImage(offer.id)
      } else {
        const created = await offersApi.create(input)
        if (image.kind === 'new') await offersApi.uploadImage(created.id, image.blob)
      }
      onDone()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save the offer.')
    } finally {
      setSaving(false)
      void qc.invalidateQueries({ queryKey: offerKeys.all })
    }
  }

  return (
    <form className="form" onSubmit={handleSubmit}>
      <p className="muted small offer-form-hint">
        An offer can be text, an image, or both. Players see whatever you add.
      </p>

      <div className="offer-image-field">
        <span className="offer-image-label">
          Image <span className="muted">(optional)</span>
        </span>
        {shownImage ? (
          <div className="offer-image-preview">
            <img src={shownImage} alt="Offer" />
            <div className="offer-image-actions">
              <button type="button" className="btn btn-secondary btn-sm" onClick={() => fileInput.current?.click()} disabled={preparing || saving}>
                <ImagePlus size={15} aria-hidden="true" />
                Change
              </button>
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={() => setImage(offer?.image_url ? { kind: 'remove' } : { kind: 'keep' })}
                disabled={saving}
              >
                <Trash2 size={15} aria-hidden="true" />
                Remove
              </button>
            </div>
          </div>
        ) : (
          <button type="button" className="offer-image-drop" onClick={() => fileInput.current?.click()} disabled={preparing || saving}>
            <ImagePlus size={26} aria-hidden="true" />
            <span>{preparing ? 'Preparing image…' : 'Add an image'}</span>
            <span className="muted small">JPEG, PNG or WebP · resized to 1600 px</span>
          </button>
        )}
        <input ref={fileInput} type="file" accept="image/jpeg,image/png,image/webp" hidden onChange={pick} />
      </div>

      <label>
        <span>
          Title <span className="muted">(optional)</span>
        </span>
        <input
          autoFocus
          maxLength={MAX_TITLE}
          placeholder="e.g. Weekend group deal"
          value={values.title}
          onChange={(e) => set('title', e.target.value)}
        />
      </label>

      <label>
        <span>
          Description <span className="muted">(optional)</span>
        </span>
        <textarea
          rows={4}
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

      {error && <p className="error">{error}</p>}

      <div className="form-actions">
        <button type="button" className="btn btn-secondary" onClick={onDone} disabled={saving}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={saving || preparing}>
          {saving ? 'Saving…' : offer ? 'Save changes' : 'Add offer'}
        </button>
      </div>
    </form>
  )
}
