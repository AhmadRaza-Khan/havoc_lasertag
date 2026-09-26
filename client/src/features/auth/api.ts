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
}
