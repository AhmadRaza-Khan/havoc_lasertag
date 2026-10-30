export type PaymentMode = 'pay_now' | 'pay_later'

export type PaymentSettings = {
  mode: PaymentMode
  /** Time allowed to pay in pay-later mode. */
  pay_later_minutes: number
  /** Fixed time allowed to pay in pay-now mode. */
  pay_now_minutes: number
  /** False when the server has no SkipCash credentials: app bookings have nothing to pay. */
  online_payment_enabled: boolean
  updated_at: string
}

export type PaymentSettingsInput = Pick<PaymentSettings, 'mode' | 'pay_later_minutes'>

export type SkipCashEnvironment = 'sandbox' | 'production'

/** Saved SkipCash credentials. Secrets are never sent back, only whether they're saved. */
export type SkipCashCredentials = {
  configured: boolean
  environment: SkipCashEnvironment | null
  client_id: string | null
  key_id: string | null
  key_secret_set: boolean
  webhook_key_set: boolean
  updated_at: string | null
}

/** Empty secrets keep the saved ones. */
export type SkipCashCredentialsInput = {
  environment: SkipCashEnvironment
  client_id: string
  key_id: string
  key_secret: string
  webhook_key: string
}

export type SkipCashTestResult = {
  ok: boolean
  checks: { name: string; ok: boolean; message: string }[]
}

/** An add-on (Bookeo option) and its picture. One row per add-on, whichever games offer it. */
export type Addon = {
  /** Same on every game (the part of Bookeo's option id after the last "_"). */
  key: string
  name: string
  kind: 'choice' | 'number' | 'on_off' | 'text'
  description: string | null
  staff_only: boolean
  /** Names of the games that offer it. */
  games: string[]
  /** API path; use `apiUrl`. */
  image_url: string | null
  image_updated_at: string | null
}
