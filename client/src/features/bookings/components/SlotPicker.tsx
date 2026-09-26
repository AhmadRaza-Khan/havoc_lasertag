import type { Slot } from '@/features/games'
import { businessDate, formatDay, formatTime } from '@/lib/format'

type SlotPickerProps = {
  slots: Slot[]
  selectedEventId: string | null
  /** Seats this booking already holds in a slot (when editing), added to what's free. */
  heldSeats?: { eventId: string; seats: number }
  onSelect: (slot: Slot) => void
}

/** Slots grouped by day (venue timezone), as selectable time buttons. */
export function SlotPicker({ slots, selectedEventId, heldSeats, onSelect }: SlotPickerProps) {
  const days = new Map<string, Slot[]>()
  for (const s of slots) {
    const day = businessDate(s.start_time)
    days.set(day, [...(days.get(day) ?? []), s])
  }

  return (
    <div className="slot-days">
      {[...days.entries()].map(([day, daySlots]) => (
        <div key={day} className="slot-day">
          <h4>{formatDay(daySlots[0].start_time)}</h4>
          <div className="slot-grid">
            {daySlots.map((s) => {
              const seats =
                s.seats_available + (heldSeats?.eventId === s.event_id ? heldSeats.seats : 0)
              const selected = s.event_id === selectedEventId
              return (
                <button
                  key={s.event_id}
                  type="button"
                  className={`slot${selected ? ' selected' : ''}`}
                  aria-pressed={selected}
                  onClick={() => onSelect(s)}
                >
                  <span className="slot-time">{formatTime(s.start_time)}</span>
                  <span className="slot-seats">
                    {seats} seat{seats === 1 ? '' : 's'}
                  </span>
                </button>
              )
            })}
          </div>
        </div>
      ))}
    </div>
  )
}
