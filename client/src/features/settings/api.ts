import { apiRequest } from '@/lib/api-client'
import type {
  PaymentSettings,
  PaymentSettingsInput,
  SkipCashCredentials,
  SkipCashCredentialsInput,
  SkipCashTestResult,
} from './types'

export const settingsApi = {
  payments: () => apiRequest<PaymentSettings>('/settings/payments'),
  savePayments: (input: PaymentSettingsInput) =>
    apiRequest<PaymentSettings>('/settings/payments', { method: 'PUT', body: JSON.stringify(input) }),
  skipcash: () => apiRequest<SkipCashCredentials>('/settings/skipcash'),
  saveSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashCredentials>('/settings/skipcash', { method: 'PUT', body: JSON.stringify(input) }),
  removeSkipcash: () => apiRequest<SkipCashCredentials>('/settings/skipcash', { method: 'DELETE' }),
  testSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashTestResult>('/settings/skipcash/test', { method: 'POST', body: JSON.stringify(input) }),
}
