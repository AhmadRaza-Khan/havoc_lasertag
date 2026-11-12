import { apiRequest } from '@/lib/api-client'
import type {
  Addon,
  StaffInput,
  StaffUser,
  PaymentSettings,
  PaymentSettingsInput,
  SkipCashCredentials,
  SkipCashCredentialsInput,
  SkipCashTestResult,
} from './types'

export const settingsApi = {
  payments: () => apiRequest<PaymentSettings>('/settings/payments'),
  savePayments: (input: PaymentSettingsInput) =>
    apiRequest<PaymentSettings>('/settings/payments', {
      method: 'PUT',
      body: JSON.stringify(input),
    }),
  skipcash: () => apiRequest<SkipCashCredentials>('/settings/skipcash'),
  saveSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashCredentials>('/settings/skipcash', {
      method: 'PUT',
      body: JSON.stringify(input),
    }),
  removeSkipcash: () => apiRequest<SkipCashCredentials>('/settings/skipcash', { method: 'DELETE' }),
  addons: () => apiRequest<Addon[]>('/settings/addons'),
  uploadAddonImage: (key: string, image: Blob) => {
    const body = new FormData()
    body.append('file', image, 'addon.jpg')
    return apiRequest<Addon[]>(`/settings/addons/${encodeURIComponent(key)}/image`, {
      method: 'PUT',
      body,
    })
  },
  removeAddonImage: (key: string) =>
    apiRequest<Addon[]>(`/settings/addons/${encodeURIComponent(key)}/image`, {
      method: 'DELETE',
    }),
  setAddonPrice: (key: string, amount: string) =>
    apiRequest<Addon[]>(`/settings/addons/${encodeURIComponent(key)}/price`, {
      method: 'PUT',
      body: JSON.stringify({ amount }),
    }),
  removeAddonPrice: (key: string) =>
    apiRequest<Addon[]>(`/settings/addons/${encodeURIComponent(key)}/price`, { method: 'DELETE' }),
  staff: () => apiRequest<StaffUser[]>('/users'),
  createStaff: (input: StaffInput) =>
    apiRequest<StaffUser>('/users', { method: 'POST', body: JSON.stringify(input) }),
  updateMe: (input: { name: string; email: string }) =>
    apiRequest<StaffUser>('/users/me', { method: 'PATCH', body: JSON.stringify(input) }),
  changeMyPassword: (current: string, next: string) =>
    apiRequest<void>('/users/me/password', {
      method: 'POST',
      body: JSON.stringify({ current_password: current, new_password: next }),
    }),
  updateStaff: (id: string, input: { name: string; email: string }) =>
    apiRequest<StaffUser>(`/users/${id}`, { method: 'PATCH', body: JSON.stringify(input) }),
  setStaffPassword: (id: string, password: string) =>
    apiRequest<void>(`/users/${id}/password`, { method: 'PUT', body: JSON.stringify({ password }) }),
  removeStaff: (id: string) => apiRequest<void>(`/users/${id}`, { method: 'DELETE' }),
  testSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashTestResult>('/settings/skipcash/test', {
      method: 'POST',
      body: JSON.stringify(input),
    }),
}
