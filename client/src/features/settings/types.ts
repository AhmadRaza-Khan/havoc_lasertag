export type PaymentMode = 'pay_now' | 'pay_later'

export type PaymentSettings = {
  mode: PaymentMode
  /** Time allowed to pay in pay-later mode. */
  pay_later_minutes: number
  /** Fixed time allowed to pay in pay-now mode. */
  pay_now_minutes: number
  /** False when the server has no QNB credentials: app bookings have nothing to pay. */
  online_payment_enabled: boolean
  updated_at: string
}

export type PaymentSettingsInput = Pick<PaymentSettings, 'mode' | 'pay_later_minutes'>
