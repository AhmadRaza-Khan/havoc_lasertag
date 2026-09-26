import { useState } from 'react'
import type { FormEvent, ReactNode } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { ArrowLeft, Info } from 'lucide-react'
import { PageHeader } from '@/components/layout/PageHeader'
import { useGames, useSlots } from '@/features/games'
import type { Game, Slot } from '@/features/games'
import {
  addDays,
  businessDate,
  daysBetween,
  formatDay,
  formatMoney,
  formatTimeRange,
} from '@/lib/format'
import { PlayerPicker } from '../components/PlayerPicker'
import type { PickedPlayer } from '../components/PlayerPicker'
import { SlotPicker } from '../components/SlotPicker'
import { StatusBadge } from '../components/StatusBadge'
import { useBooking, useCreateBooking, useUpdateBooking } from '../hooks'
import type { Booking } from '../types'
import '../bookings.css'

/** Bookeo allows at most 31 days per availability search. */
const MAX_RANGE_DAYS = 31
const DEFAULT_RANGE_DAYS = 7

/** /bookings/new and /bookings/:id/edit */
export default function BookingFormPage() {
  const { id } = useParams()
  const booking = useBooking(id)

  const header = (
    <PageHeader
      title={id ? 'Edit booking' : 'New booking'}
      description={
        id
          ? 'Move the booking to another slot or change the number of people.'
          : 'Book a slot for a player. The booking is created in Bookeo.'
      }
      actions={
        <Link to="/bookings" className="btn btn-secondary">
          <ArrowLeft size={16} aria-hidden="true" />
          Back to bookings
        </Link>
      }
    />
  )

  if (id && booking.isPending) return <>{header}<p className="muted">Loading booking…</p></>
  if (id && booking.isError) return <>{header}<p className="error">{booking.error.message}</p></>

  return (
    <>
      {header}
      {/* Keyed so state initializes from the loaded booking */}
      <BookingForm key={id ?? 'new'} booking={booking.data ?? null} />
    </>
  )
}

function BookingForm({ booking }: { booking: Booking | null }) {
  const isEdit = booking !== null
  const navigate = useNavigate()
  const today = businessDate()
  const initialFrom = booking ? maxDate(today, businessDate(booking.start_time)) : today

  const [player, setPlayer] = useState<PickedPlayer | null>(booking?.player ?? null)
  const [productId, setProductId] = useState<string | null>(booking?.product_id ?? null)
  const [from, setFrom] = useState(initialFrom)
  const [to, setTo] = useState(addDays(initialFrom, DEFAULT_RANGE_DAYS - 1))
  // When editing, the booking's own slot is pre-selected even if it's not in the search results.
  const [slot, setSlot] = useState<Slot | null>(
    booking
      ? {
          event_id: booking.event_id,
          start_time: booking.start_time,
          end_time: booking.end_time,
          seats_available: 0,
        }
      : null,
  )
  const [numPeople, setNumPeople] = useState(String(booking?.num_people ?? ''))

  const games = useGames()
  const game = games.data?.find((g) => g.product_id === productId) ?? null

  const rangeError =
    to < from
      ? '"To" must be on or after "From".'
      : daysBetween(from, to) + 1 > MAX_RANGE_DAYS
        ? `Pick at most ${MAX_RANGE_DAYS} days.`
        : null
  const slots = useSlots(rangeError ? null : productId, from, to)

  const create = useCreateBooking()
  const update = useUpdateBooking()
  const mutation = isEdit ? update : create

  // Seats this booking already holds count as available for itself.
  const heldSeats = booking ? { eventId: booking.event_id, seats: booking.num_people } : undefined
  const slotCapacity = slot
    ? slot.event_id === booking?.event_id
      ? (slots.data?.slots.find((s) => s.event_id === slot.event_id)?.seats_available ?? 0) +
        booking.num_people
      : slot.seats_available
    : null

  const people = Number(numPeople)
  const peopleError = validatePeople(people, numPeople, game, slotCapacity)
  const canSubmit =
    player !== null && game !== null && slot !== null && !peopleError && !mutation.isPending

  function selectGame(value: string) {
    setProductId(value || null)
    setSlot(null)
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!canSubmit || !player || !game || !slot) return
    if (booking) {
      update.mutate(
        { id: booking.id, input: { event_id: slot.event_id, num_people: people } },
        {
          onSuccess: (b) =>
            navigate('/bookings', { state: { flash: `Booking #${b.booking_number} updated.` } }),
        },
      )
    } else {
      create.mutate(
        { player_id: player.id, product_id: game.product_id, event_id: slot.event_id, num_people: people },
        {
          onSuccess: (b) =>
            navigate('/bookings', {
              state: {
                flash: `Booking #${b.booking_number} created for ${b.player.full_name}${
                  b.status === 'pending' ? ' (pending staff approval in Bookeo)' : ''
                }.`,
              },
            }),
        },
      )
    }
  }

  const otherSlots = slots.data?.slots.filter((s) => s.event_id !== booking?.event_id) ?? []

  return (
    <form className="booking-form" onSubmit={handleSubmit}>
      <div className="booking-steps">
        <Step n={1} title="Player">
          {booking ? (
            <ReadOnly
              primary={booking.player.full_name}
              secondary={`@${booking.player.playername}`}
            />
          ) : (
            <PlayerPicker value={player} onChange={setPlayer} />
          )}
        </Step>

        <Step n={2} title="Game">
          {booking ? (
            <ReadOnly
              primary={booking.game_name}
              secondary="To change the game, cancel this booking and create a new one."
            />
          ) : games.isPending ? (
            <p className="muted">Loading games…</p>
          ) : games.isError ? (
            <p className="error">{games.error.message}</p>
          ) : (
            <select
              aria-label="Game"
              value={productId ?? ''}
              onChange={(e) => selectGame(e.target.value)}
            >
              <option value="">Select a game…</option>
              {games.data.map((g) => (
                <option key={g.product_id} value={g.product_id}>
                  {g.name}
                </option>
              ))}
            </select>
          )}
          {game && <GameFacts game={game} />}
        </Step>

        <Step n={3} title="Dates">
          <div className="form-row">
            <label>
              From
              <input
                type="date"
                value={from}
                min={today}
                required
                onChange={(e) => {
                  const f = e.target.value
                  setFrom(f)
                  if (f && to < f) setTo(addDays(f, DEFAULT_RANGE_DAYS - 1))
                }}
              />
            </label>
            <label>
              To
              <input
                type="date"
                value={to}
                min={from}
                max={addDays(from, MAX_RANGE_DAYS - 1)}
                required
                onChange={(e) => setTo(e.target.value)}
              />
            </label>
          </div>
          {rangeError && <p className="error">{rangeError}</p>}
        </Step>

        <Step n={4} title="Slot">
          {booking && (
            <div className="current-slot">
              <span className="muted small">Current slot</span>
              <button
                type="button"
                className={`slot slot-wide${slot?.event_id === booking.event_id ? ' selected' : ''}`}
                aria-pressed={slot?.event_id === booking.event_id}
                onClick={() =>
                  setSlot({
                    event_id: booking.event_id,
                    start_time: booking.start_time,
                    end_time: booking.end_time,
                    seats_available: 0,
                  })
                }
              >
                <span className="slot-time">
                  {formatDay(booking.start_time)} ·{' '}
                  {formatTimeRange(booking.start_time, booking.end_time)}
                </span>
                <span className="slot-seats">Keep this slot</span>
              </button>
            </div>
          )}
          {!productId ? (
            <p className="muted">Select a game to see its available slots.</p>
          ) : rangeError ? null : slots.isPending ? (
            <p className="muted">Loading slots…</p>
          ) : slots.isError ? (
            <p className="error">{slots.error.message}</p>
          ) : otherSlots.length === 0 ? (
            <p className="muted">
              No {isEdit ? 'other ' : ''}available slots between {formatDay(`${from}T12:00:00Z`)} and{' '}
              {formatDay(`${to}T12:00:00Z`)}.
            </p>
          ) : (
            <>
              {isEdit && <span className="muted small">Or move to</span>}
              <SlotPicker
                slots={otherSlots}
                selectedEventId={slot?.event_id ?? null}
                heldSeats={heldSeats}
                onSelect={setSlot}
              />
            </>
          )}
        </Step>

        <Step n={5} title="Number of people">
          <label className="people-input">
            <span className="sr-only">Number of people</span>
            <input
              type="number"
              inputMode="numeric"
              min={game?.min_people ?? 1}
              max={Math.min(game?.max_people ?? 100, slotCapacity ?? 100)}
              value={numPeople}
              onChange={(e) => setNumPeople(e.target.value)}
              placeholder={game?.min_people ? `${game.min_people}–${game.max_people}` : ''}
            />
          </label>
          {numPeople !== '' && peopleError && <p className="error">{peopleError}</p>}
        </Step>
      </div>

      <aside className="booking-summary panel panel-padded">
        <h2>Summary</h2>
        {booking && (
          <div className="summary-status">
            <span className="mono">#{booking.booking_number}</span>
            <StatusBadge status={booking.status} />
          </div>
        )}
        <dl>
          <dt>Player</dt>
          <dd>{player ? player.full_name : <span className="muted">Not selected</span>}</dd>
          <dt>Game</dt>
          <dd>{game?.name ?? booking?.game_name ?? <span className="muted">Not selected</span>}</dd>
          <dt>Slot</dt>
          <dd>
            {slot ? (
              <>
                {formatDay(slot.start_time)}
                <br />
                {formatTimeRange(slot.start_time, slot.end_time)}
              </>
            ) : (
              <span className="muted">Not selected</span>
            )}
          </dd>
          <dt>People</dt>
          <dd>{numPeople && !peopleError ? people : <span className="muted">—</span>}</dd>
          <dt>Estimated price</dt>
          <dd>{estimate(game, people, peopleError) ?? <span className="muted">—</span>}</dd>
        </dl>
        {game?.requires_approval && !isEdit && (
          <p className="summary-note">
            <Info size={16} aria-hidden="true" />
            This game needs staff approval: the booking starts as pending in Bookeo.
          </p>
        )}

        {mutation.error && <p className="error">{mutation.error.message}</p>}

        <button type="submit" className="btn btn-primary" disabled={!canSubmit}>
          {mutation.isPending ? 'Saving…' : isEdit ? 'Save changes' : 'Create booking'}
        </button>
        <p className="muted small">Final price is calculated by Bookeo.</p>
      </aside>
    </form>
  )
}

function Step({ n, title, children }: { n: number; title: string; children: ReactNode }) {
  return (
    <section className="booking-step panel panel-padded">
      <h3>
        <span className="step-number">{n}</span>
        {title}
      </h3>
      {children}
    </section>
  )
}

function ReadOnly({ primary, secondary }: { primary: string; secondary: string }) {
  return (
    <div className="stack-cell">
      <span className="strong">{primary}</span>
      <span className="muted small">{secondary}</span>
    </div>
  )
}

function GameFacts({ game }: { game: Game }) {
  return (
    <ul className="game-facts">
      {game.duration_minutes && <li>{game.duration_minutes} min</li>}
      {game.price && (
        <li>
          {formatMoney(game.price.amount, game.price.currency)}
          {game.price.per_person ? ' / person' : ' / booking'}
        </li>
      )}
      {game.min_people !== null && (
        <li>
          {game.min_people}–{game.max_people} people
        </li>
      )}
      {game.requires_approval && <li>Needs staff approval</li>}
    </ul>
  )
}

function validatePeople(
  people: number,
  raw: string,
  game: Game | null,
  slotCapacity: number | null,
): string | null {
  if (raw === '' || !Number.isInteger(people) || people < 1) return 'Enter the number of people.'
  if (game?.min_people != null && game.max_people != null) {
    if (people < game.min_people || people > game.max_people)
      return `This game is for ${game.min_people}–${game.max_people} people.`
  }
  if (slotCapacity !== null && slotCapacity > 0 && people > slotCapacity)
    return `Only ${slotCapacity} seat${slotCapacity === 1 ? '' : 's'} left in this slot.`
  return null
}

function estimate(game: Game | null, people: number, peopleError: string | null): string | null {
  if (!game?.price) return null
  const unit = Number(game.price.amount)
  if (!Number.isFinite(unit)) return null
  if (game.price.per_person) {
    if (peopleError) return null
    return formatMoney(String(unit * people), game.price.currency)
  }
  return formatMoney(game.price.amount, game.price.currency)
}

function maxDate(a: string, b: string): string {
  return a > b ? a : b
}
