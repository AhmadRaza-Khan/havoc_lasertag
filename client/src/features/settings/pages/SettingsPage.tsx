import { useState } from 'react'
import type { FormEvent } from 'react'
import { CircleCheck, TriangleAlert } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { AddonImagesSettings } from '../components/AddonImagesSettings'
import { SkipCashSettings } from '../components/SkipCashSettings'
import { usePaymentSettings, useSavePaymentSettings } from '../hooks'
import type { PaymentMode, PaymentSettings } from '../types'
import '../settings.css'

type Unit = 'minutes' | 'hours' | 'days'
const UNIT_MINUTES: Record<Unit, number> = { minutes: 1, hours: 60, days: 1440 }
const MIN_MINUTES = 15
const MAX_MINUTES = 30 * 1440

/** Shows a stored number of minutes in the largest unit that divides it. */
function splitMinutes(total: number): { value: number; unit: Unit } {
  if (total % 1440 === 0) return { value: total / 1440, unit: 'days' }
  if (total % 60 === 0) return { value: total / 60, unit: 'hours' }
  return { value: total, unit: 'minutes' }
}

function describe(minutes: number) {
  const { value, unit } = splitMinutes(minutes)
  return `${value} ${value === 1 ? unit.slice(0, -1) : unit}`
}

export default function SettingsPage() {
  const { data, isPending, isError, error } = usePaymentSettings()

  return (
    <>
      <PageHeader title="Settings" description="How bookings made in the mobile app are paid." />
      <div className="panel panel-padded">
        {isPending ? (
          <p className="muted">Loading settings…</p>
        ) : isError ? (
          <p className="error">{error.message}</p>
        ) : (
          <PaymentSettingsForm settings={data} />
        )}
      </div>
      <div className="panel panel-padded settings-panel">
        <SkipCashSettings />
      </div>
      <div className="panel panel-padded settings-panel">
        <AddonImagesSettings />
      </div>
    </>
  )
}

function PaymentSettingsForm({ settings }: { settings: PaymentSettings }) {
  const initial = splitMinutes(settings.pay_later_minutes)
  const [mode, setMode] = useState<PaymentMode>(settings.mode)
  const [value, setValue] = useState(String(initial.value))
  const [unit, setUnit] = useState<Unit>(initial.unit)
  const [saved, setSaved] = useState(false)
  const save = useSavePaymentSettings()

  const minutes = Math.round(Number(value) * UNIT_MINUTES[unit])
  const validTime = Number.isFinite(minutes) && minutes >= MIN_MINUTES && minutes <= MAX_MINUTES

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    setSaved(false)
    save.mutate(
      // Pay-now leaves the pay-later time as it was, ready for when it's switched back on.
      { mode, pay_later_minutes: mode === 'pay_later' ? minutes : settings.pay_later_minutes },
      { onSuccess: () => setSaved(true) },
    )
  }

  return (
    <form className="settings-section" onSubmit={handleSubmit}>
      <h2>Payments</h2>
      <p className="muted">Customers pay by card on SkipCash's secure payment page inside the app.</p>

      {!settings.online_payment_enabled && (
        <p className="notice notice-warning" role="status">
          <TriangleAlert size={16} aria-hidden="true" />
          Online payment is off: SkipCash isn't set up yet. App bookings have nothing to pay until
          its credentials are saved below.
        </p>
      )}

      <fieldset className="choice-list">
        <legend className="sr-only">When customers pay</legend>
        <label className="choice">
          <input
            type="radio"
            name="mode"
            value="pay_now"
            checked={mode === 'pay_now'}
            onChange={() => setMode('pay_now')}
          />
          <span className="choice-title">Pay now</span>
          <span className="muted">
            The seat is booked and the customer pays right away. If they leave the payment page
            without paying, the booking is canceled immediately (or after{' '}
            {settings.pay_now_minutes} minutes if the app is closed mid-payment).
          </span>
        </label>

        <div className="choice-group">
          <label className="choice">
            <input
              type="radio"
              name="mode"
              value="pay_later"
              checked={mode === 'pay_later'}
              onChange={() => setMode('pay_later')}
            />
            <span className="choice-title">Pay later</span>
            <span className="muted">
              Customers get an email with a payment link and can pay any time within the time you
              set. If it isn't paid by then, the booking is canceled.
            </span>
          </label>
          {mode === 'pay_later' && (
            <div className="choice-extra">
              <label>
                Time to pay
                <input
                  type="number"
                  required
                  min={1}
                  step={1}
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  aria-invalid={!validTime}
                />
              </label>
              <select aria-label="Unit" value={unit} onChange={(e) => setUnit(e.target.value as Unit)}>
                <option value="minutes">minutes</option>
                <option value="hours">hours</option>
                <option value="days">days</option>
              </select>
            </div>
          )}
        </div>
      </fieldset>

      {mode === 'pay_later' && !validTime && (
        <p className="error">Time to pay must be between 15 minutes and 30 days.</p>
      )}

      <ul className="settings-list muted small">
        <li>A booking is never left unpaid past the start of its game.</li>
        <li>
          Customers can pay in full or in part (at least QAR 120). Once something is paid the
          booking is kept: the rest can be paid in the app, also by the other players in the
          group, until the game starts, and after that at the venue.
        </li>
        <li>
          Customers are emailed when payment is due (pay later), when it's received, and when an
          unpaid booking is canceled.
        </li>
        <li>Changes apply to new bookings; existing ones keep their deadline.</li>
        <li>Bookings made by staff here are paid at the venue.</li>
      </ul>

      {save.error && <p className="error">{save.error.message}</p>}

      <div className="form-actions settings-actions">
        {saved && !save.isPending && (
          <span className="notice notice-success saved-note" role="status">
            <CircleCheck size={16} aria-hidden="true" />
            Saved.{' '}
            {mode === 'pay_now'
              ? 'Customers pay right after booking.'
              : `Customers have ${describe(minutes)} to pay.`}
          </span>
        )}
        <button
          type="submit"
          className="btn btn-primary"
          disabled={save.isPending || (mode === 'pay_later' && !validTime)}
        >
          {save.isPending ? 'Saving…' : 'Save'}
        </button>
      </div>
    </form>
  )
}
