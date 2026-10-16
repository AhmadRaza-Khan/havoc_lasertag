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
  formatDay,
  formatMoney,
  formatTimeRange,
} from '@/lib/format'
import { addOnLines, addOnsError, initialAddOnValue, toOptionInputs } from '../addOns'
import type { AddOnValues } from '../addOns'
import { AddOnsPicker } from '../components/AddOns'
import { OtherPlayers } from '../components/OtherPlayers'
import { PlayerPicker } from '../components/PlayerPicker'
import type { PickedPlayer } from '../components/PlayerPicker'
import { SlotPicker } from '../components/SlotPicker'
import { StatusBadge } from '../components/StatusBadge'
import { useBooking, useCreateBooking, useUpdateBooking } from '../hooks'
import type { Booking } from '../types'
import '../bookings.css'

/** Bookeo allows at most 31 days per availability search. */

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
  const [date, setDate] = useState(initialFrom)
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
  const [others, setOthers] = useState<PickedPlayer[]>(booking?.other_players ?? [])
  const [numPeople, setNumPeople] = useState(String(booking?.num_people ?? ''))

  // The group is the player plus the other players (plus any unregistered guests), so the
  // number of people follows the list: adding a player raises it to fit, removing one lowers
  // it by one. The admin can still type a larger number for guests.
  function addOther(p: PickedPlayer) {
    const next = [...others, p]
    setOthers(next)
    setNumPeople((n) => String(Math.max(Number(n) || 0, next.length + 1)))
  }
  function removeOther(p: PickedPlayer) {
    const next = others.filter((o) => o.id !== p.id)
    setOthers(next)
    setNumPeople((n) => String(Math.max(next.length + 1, (Number(n) || 1) - 1)))
  }
  function selectPlayer(p: PickedPlayer | null) {
    setPlayer(p)
    // The booking's player can't also be listed among the others.
    if (p && others.some((o) => o.id === p.id)) removeOther(p)
  }

  const games = useGames()
  const game = games.data?.find((g) => g.product_id === productId) ?? null

  // Add-ons: what the admin changed, over the booking's saved ones (or Bookeo's defaults).
  const [addOnEdits, setAddOnEdits] = useState<AddOnValues>({})
  const gameOptions = game?.options ?? []
  const addOnValues: AddOnValues = Object.fromEntries(
    gameOptions.map((o) => [o.id, addOnEdits[o.id] ?? initialAddOnValue(o, booking?.options ?? null)]),
  )
  const addOnProblem = addOnsError(gameOptions, addOnValues)
  const chosenAddOns = addOnLines(gameOptions, addOnValues)

  // The chosen day's slots.
  const slots = useSlots(date ? productId : null, date, date)

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
  const peopleError = validatePeople(people, numPeople, game, slotCapacity, others.length)
  const canSubmit =
    player !== null &&
    game !== null &&
    slot !== null &&
    !peopleError &&
    !addOnProblem &&
    !mutation.isPending

  function selectGame(value: string) {
    setProductId(value || null)
    setSlot(null)
    setAddOnEdits({})
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!canSubmit || !player || !game || !slot) return
    if (booking) {
      update.mutate(
        {
          id: booking.id,
          input: {
            event_id: slot.event_id,
            num_people: people,
            other_player_ids: others.map((o) => o.id),
            options: toOptionInputs(gameOptions, addOnValues),
          },
        },
        {
          onSuccess: (b) =>
            navigate('/bookings', { state: { flash: `Booking #${b.booking_number} updated.` } }),
        },
      )
    } else {
      create.mutate(
        {
          player_id: player.id,
          product_id: game.product_id,
          event_id: slot.event_id,
          num_people: people,
          other_player_ids: others.map((o) => o.id),
          options: toOptionInputs(gameOptions, addOnValues),
        },
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

  // Only the chosen day (venue time), and not the slot the booking already has.
  const otherSlots =
    slots.data?.slots.filter((s) => s.event_id !== booking?.event_id && businessDate(s.start_time) === date) ?? []

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
            <PlayerPicker value={player} onChange={selectPlayer} />
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
        <Step n={3} title="Add-ons" optional>
          {!game ? (
            <p className="muted">Select a game to see its add-ons.</p>
          ) : gameOptions.length === 0 ? (
            <p className="muted">This game has no add-ons.</p>
          ) : (
            <>
              <AddOnsPicker
                options={gameOptions}
                values={addOnValues}
                onChange={(id, value) => setAddOnEdits((e) => ({ ...e, [id]: value }))}
              />
              {addOnProblem && <p className="error">{addOnProblem}</p>}
            </>
          )}
        </Step>
        <Step n={4} title="Number of players">
          <label className="people-input">
            <span className="sr-only">Number of people</span>
            <input
              type="number"
              inputMode="numeric"
              min={Math.max(game?.min_people ?? 1, others.length + 1)}
              max={Math.min(game?.max_people ?? 100, slotCapacity ?? 100)}
              value={numPeople}
              onChange={(e) => setNumPeople(e.target.value)}
              placeholder={game?.min_people ? `${game.min_people}–${game.max_people}` : ''}
            />
          </label>
          {others.length > 0 && (
            <p className="muted small">
              Includes {player?.full_name ?? 'the player'} and {others.length} other player
              {others.length === 1 ? '' : 's'}. Raise it for unregistered guests.
            </p>
          )}
          {numPeople !== '' && peopleError && <p className="error">{peopleError}</p>}
        </Step>
        <Step n={5} title="Other players" optional>
          <p className="muted small">
            Registered players joining this booking. Each one is counted in the number of players.
          </p>
          <OtherPlayers
            value={others}
            mainPlayerId={player?.id ?? null}
            onAdd={addOther}
            onRemove={removeOther}
          />
        </Step>
        <Step n={6} title="Date">
          <div className="date-picker">
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={date <= today}
              onClick={() => setDate(maxDate(today, addDays(date, -1)))}
            >
              ‹ Previous day
            </button>
            <input
              type="date"
              aria-label="Date"
              value={date}
              min={today}
              required
              onChange={(e) => e.target.value && setDate(e.target.value)}
            />
            <button type="button" className="btn btn-secondary btn-sm" onClick={() => setDate(addDays(date, 1))}>
              Next day ›
            </button>
          </div>
          <p className="muted small">{formatDay(`${date}T12:00:00Z`)}</p>
        </Step>
        <Step n={7} title="Time">
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
            <p className="muted">Select a game to see its available times.</p>
          ) : slots.isPending || slots.isPlaceholderData ? (
            // isPlaceholderData: still showing the previous day's (or game's) results.
            <p className="muted">Loading times…</p>
          ) : slots.isError ? (
            <p className="error">{slots.error.message}</p>
          ) : otherSlots.length === 0 ? (
            <p className="muted">
              No {isEdit ? 'other ' : ''}available times on {formatDay(`${date}T12:00:00Z`)}. Try another day.
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
          {others.length > 0 && (
            <>
              <dt>With</dt>
              <dd>{others.map((o) => o.full_name).join(', ')}</dd>
            </>
          )}
          <dt>Game</dt>
          <dd>{game?.name ?? booking?.game_name ?? <span className="muted">Not selected</span>}</dd>
          <dt>Date &amp; time</dt>
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
          <dt>Players</dt>
          <dd>{numPeople && !peopleError ? people : <span className="muted">—</span>}</dd>
          {chosenAddOns.length > 0 && (
            <>
              <dt>Add-ons</dt>
              <dd>
                {chosenAddOns.map((line) => (
                  <span key={line} className="summary-addon">
                    {line}
                  </span>
                ))}
              </dd>
            </>
          )}
          <dt>Estimated price</dt>
          <dd>
            {estimate(game, people, peopleError) ?? <span className="muted">—</span>}
            {chosenAddOns.length > 0 && <span className="muted small summary-addon">+ add-ons</span>}
          </dd>
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
        <p className="muted small">Final price, including add-ons, is calculated by Bookeo.</p>
      </aside>
    </form>
  )
}

function Step({
  n,
  title,
  optional = false,
  children,
}: {
  n: number
  title: string
  optional?: boolean
  children: ReactNode
}) {
  return (
    <section className="booking-step panel panel-padded">
      <h3>
        <span className="step-number">{n}</span>
        {title}
        {optional && <span className="muted small step-optional">(optional)</span>}
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
  others: number,
): string | null {
  if (raw === '' || !Number.isInteger(people) || people < 1) return 'Enter the number of people.'
  if (people < others + 1)
    return `At least ${others + 1}: the player plus ${others} other player${others === 1 ? '' : 's'}.`
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
