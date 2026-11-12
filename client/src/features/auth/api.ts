import { apiRequest } from '@/lib/api-client'
import type { LoginResponse, User } from './types'

export const authApi = {
  login: (username: string, password: string) =>
    apiRequest<LoginResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () => apiRequest<void>('/auth/logout', { method: 'POST' }),
  me: () => apiRequest<User>('/auth/me'),
  forgotPassword: (login: string) =>
    apiRequest<void>('/auth/forgot-password', { method: 'POST', body: JSON.stringify({ login }) }),
  resetPassword: (login: string, code: string, newPassword: string) =>
    apiRequest<void>('/auth/reset-password', {
      method: 'POST',
      body: JSON.stringify({ login, code, new_password: newPassword }),
    }),
}
