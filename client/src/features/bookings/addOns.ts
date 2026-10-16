import type { GameOption } from '@/features/games'
import type { BookingOption, OptionInput } from './types'

// Add-ons (Bookeo "options") for the booking form: defaults, validation, summary, request body.

/** Chosen values by option id ("" = nothing chosen). */
export type AddOnValues = Record<string, string>

/**
 * The value to start with: what the booking already has (when editing), else Bookeo's default.
 * Bookings store a choice by the value's name; the form works with value ids.
 */
export function initialAddOnValue(option: GameOption, existing: BookingOption[] | null): string {
  const saved = existing?.find((o) => o.id === option.id)
  if (saved) {
    if (option.kind === 'choice') {
      return option.values.find((v) => v.name.trim() === saved.value.trim())?.id ?? ''
    }
    return saved.value
  }
  // An existing booking without this add-on: leave it off rather than applying a default.
  if (existing) return option.kind === 'on_off' ? 'false' : ''
  return option.default ?? (option.kind === 'on_off' ? 'false' : '')
}

/** What to send: chosen choices, every on/off, filled-in numbers and text. */
export function toOptionInputs(options: GameOption[], values: AddOnValues): OptionInput[] {
  return options.flatMap((o) => {
    const value = (values[o.id] ?? '').trim()
    if (o.kind === 'on_off') return [{ id: o.id, value: value === 'true' ? 'true' : 'false' }]
    return value ? [{ id: o.id, value }] : []
  })
}

/** One line per chosen add-on, for summaries. */
export function addOnLines(options: GameOption[], values: AddOnValues): string[] {
  return options.flatMap((o) => {
    const v = values[o.id] ?? ''
    if (o.kind === 'on_off') return v === 'true' ? [o.name] : []
    if (!v) return []
    if (o.kind === 'choice') {
      const choice = o.values.find((x) => x.id === v)
      return choice ? [`${o.name}: ${choice.name}`] : []
    }
    return [`${o.name}: ${v}`]
  })
}

/** Problems that block saving (required text add-ons, numbers out of range). */
export function addOnsError(options: GameOption[], values: AddOnValues): string | null {
  for (const o of options) {
    const v = (values[o.id] ?? '').trim()
    if (o.kind === 'text' && o.required && !v) return `“${o.name}” is required.`
    if (o.kind === 'number' && v !== '') {
      const n = Number(v)
      if (!Number.isInteger(n) || (o.min !== null && n < o.min) || (o.max !== null && n > o.max)) {
        return `“${o.name}” must be a whole number from ${o.min} to ${o.max}.`
      }
    }
  }
  return null
}
