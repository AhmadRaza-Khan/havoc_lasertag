import { useRef, useState } from 'react'
import type { ChangeEvent, FormEvent } from 'react'
import { ImageOff, ImagePlus, RefreshCw, Trash2 } from 'lucide-react'
import { ConfirmDialog } from '@/components/ui/ConfirmDialog'
import { prepareImage } from '@/features/offers'
import { apiUrl } from '@/lib/api-client'
import {
  useAddons,
  useRemoveAddonImage,
  useRemoveAddonPrice,
  useSetAddonPrice,
  useUploadAddonImage,
} from '../hooks'
import type { Addon } from '../types'

/**
 * A picture and a price for each add-on, shown next to the add-on when booking. One of each
 * covers the add-on on every game. The price is for reference: Bookeo calculates what a booking
 * costs.
 */
export function AddonImagesSettings() {
  const { data, isPending, isError, error } = useAddons()
  const [toRemove, setToRemove] = useState<Addon | null>(null)
  const remove = useRemoveAddonImage()

  return (
    <section className="settings-section">
      <h2>Add-on pictures and prices</h2>
      <p className="muted">
        Shown next to each add-on when booking; one picture and price cover the add-on on every game
        that offers it. Prices here are for display: the amount charged is still the total Bookeo
        calculates.
      </p>

      {isPending ? (
        <p className="muted">Loading add-ons…</p>
      ) : isError ? (
        <p className="error">{error.message}</p>
      ) : data.length === 0 ? (
        <p className="muted">No add-ons in Bookeo yet.</p>
      ) : (
        <ul className="addon-images">
          {data.map((a) => (
            <AddonRow key={a.key} addon={a} onRemove={() => setToRemove(a)} />
          ))}
        </ul>
      )}

      <ConfirmDialog
        open={toRemove !== null}
        title="Remove picture"
        message={
          <>
            Remove the picture of <strong>{toRemove?.name}</strong>? The add-on stays; it's just
            shown without a picture.
          </>
        }
        confirmLabel="Remove"
        pending={remove.isPending}
        error={remove.error?.message}
        onConfirm={() =>
          toRemove && remove.mutate(toRemove.key, { onSuccess: () => setToRemove(null) })
        }
        onClose={() => {
          setToRemove(null)
          remove.reset()
        }}
      />
    </section>
  )
}

function AddonRow({ addon, onRemove }: { addon: Addon; onRemove: () => void }) {
  const fileInput = useRef<HTMLInputElement>(null)
  const upload = useUploadAddonImage()
  const [preparing, setPreparing] = useState(false)
  const [prepError, setPrepError] = useState<string | null>(null)
  const busy = preparing || upload.isPending
  const setPrice = useSetAddonPrice()
  const removePrice = useRemoveAddonPrice()
  const [price, setPriceText] = useState(addon.price ?? '')
  const priceBusy = setPrice.isPending || removePrice.isPending
  const validPrice = /^\d{1,9}(\.\d{1,2})?$/.test(price.trim())
  const priceChanged = validPrice && Number(price) !== Number(addon.price ?? NaN)

  function savePrice(e: FormEvent) {
    e.preventDefault()
    if (!priceChanged || priceBusy) return
    removePrice.reset()
    setPrice.mutate({ key: addon.key, amount: price.trim() })
  }

  function clearPrice() {
    setPrice.reset()
    removePrice.mutate(addon.key, { onSuccess: () => setPriceText('') })
  }

  async function pick(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    e.target.value = '' // picking the same file again still fires
    if (!file) return
    setPrepError(null)
    upload.reset()
    setPreparing(true)
    try {
      const image = await prepareImage(file)
      upload.mutate({ key: addon.key, image })
    } catch (err) {
      setPrepError(err instanceof Error ? err.message : 'Could not read the image.')
    } finally {
      setPreparing(false)
    }
  }

  const error =
    prepError ?? upload.error?.message ?? setPrice.error?.message ?? removePrice.error?.message
  return (
    <li className="addon-image-row">
      <div className="addon-image-frame">
        {addon.image_url ? (
          <img src={apiUrl(addon.image_url)} alt="" />
        ) : (
          <ImageOff size={22} aria-hidden="true" className="muted" />
        )}
      </div>
      <div className="addon-image-info">
        <span className="strong">{addon.name}</span>
        <span className="muted small">
          {addon.games.length === 1 ? addon.games[0] : `${addon.games.length} games`}
          {addon.staff_only && ' · staff only'}
        </span>
        <form className="addon-price-form" onSubmit={savePrice}>
          <label>
            <span className="muted small">Price (QAR)</span>
            <input
              inputMode="decimal"
              placeholder="Not set"
              value={price}
              onChange={(e) => setPriceText(e.target.value)}
              aria-label={`Price of ${addon.name} in QAR`}
              aria-invalid={price.trim() !== '' && !validPrice}
            />
          </label>
          <button
            type="submit"
            className="btn btn-secondary btn-sm"
            disabled={!priceChanged || priceBusy}
          >
            {setPrice.isPending ? 'Saving…' : addon.price ? 'Update' : 'Add price'}
          </button>
          {addon.price && (
            <button
              type="button"
              className="btn btn-secondary btn-sm addon-image-remove"
              disabled={priceBusy}
              onClick={clearPrice}
              aria-label={`Remove the price of ${addon.name}`}
            >
              Remove
            </button>
          )}
        </form>
        {error && <span className="error small">{error}</span>}
      </div>
      <div className="addon-image-actions">
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={busy}
          onClick={() => fileInput.current?.click()}
        >
          {addon.image_url ? (
            <RefreshCw size={14} aria-hidden="true" />
          ) : (
            <ImagePlus size={14} aria-hidden="true" />
          )}
          {busy ? 'Uploading…' : addon.image_url ? 'Replace' : 'Add picture'}
        </button>
        {addon.image_url && (
          <button
            type="button"
            className="btn btn-secondary btn-sm addon-image-remove"
            disabled={busy}
            onClick={onRemove}
            aria-label={`Remove the picture of ${addon.name}`}
          >
            <Trash2 size={14} aria-hidden="true" />
            Remove
          </button>
        )}
        <input
          ref={fileInput}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          hidden
          onChange={pick}
        />
      </div>
    </li>
  )
}
