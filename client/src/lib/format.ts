import { BUSINESS_TIMEZONE } from './config'

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
})

export function formatDate(iso: string): string {
  return dateFormatter.format(new Date(iso))
}

// Booking times are always shown in the venue's timezone.

const dayFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: 'short',
  day: 'numeric',
  month: 'short',
  year: 'numeric',
  timeZone: BUSINESS_TIMEZONE,
})

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
  timeZone: BUSINESS_TIMEZONE,
})

/** e.g. "Sat, 27 Sep 2026" */
export function formatDay(iso: string): string {
  return dayFormatter.format(new Date(iso))
}

/** e.g. "14:00" */
export function formatTime(iso: string): string {
  return timeFormatter.format(new Date(iso))
}

/** e.g. "14:00–15:00" */
export function formatTimeRange(startIso: string, endIso: string): string {
  return `${formatTime(startIso)}–${formatTime(endIso)}`
}

/** YYYY-MM-DD of an instant in the venue's timezone. */
export function businessDate(iso: string | Date = new Date()): string {
  // en-CA formats as YYYY-MM-DD
  return new Intl.DateTimeFormat('en-CA', { timeZone: BUSINESS_TIMEZONE }).format(new Date(iso))
}

/** Adds days to a YYYY-MM-DD date. */
export function addDays(date: string, days: number): string {
  const d = new Date(`${date}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() + days)
  return d.toISOString().slice(0, 10)
}

/** Whole days from `from` to `to` (YYYY-MM-DD). */
export function daysBetween(from: string, to: string): number {
  return Math.round((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 86_400_000)
}

export function formatMoney(amount: string, currency: string): string {
  const n = Number(amount)
  return Number.isFinite(n) ? `${n.toLocaleString()} ${currency}` : `${amount} ${currency}`
}
