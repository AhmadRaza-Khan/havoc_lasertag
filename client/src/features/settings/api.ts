import { apiRequest } from '@/lib/api-client'
import type { PaymentSettings, PaymentSettingsInput } from './types'

export const settingsApi = {
  payments: () => apiRequest<PaymentSettings>('/settings/payments'),
  savePayments: (input: PaymentSettingsInput) =>
    apiRequest<PaymentSettings>('/settings/payments', { method: 'PUT', body: JSON.stringify(input) }),
}
