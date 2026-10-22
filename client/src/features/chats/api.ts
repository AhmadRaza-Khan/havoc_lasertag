import { apiRequest, toQueryString } from '@/lib/api-client'
import type { Paginated } from '@/lib/api-client'
import type { ChatMessage, ChatPlayer, ChatReport, Conversation, MessagesPage } from './types'

// Staff chat with players as the "Havoc Admin" player.
export const chatsApi = {
  conversations: () => apiRequest<Conversation[]>('/chats/conversations'),
  players: (search: string, page: number) =>
    apiRequest<Paginated<ChatPlayer>>(`/chats/players${toQueryString({ search, page, per_page: 50 })}`),
  messages: (playerId: string, before?: string) =>
    apiRequest<MessagesPage>(`/chats/conversations/${playerId}/messages${toQueryString({ before, limit: 50 })}`),
  send: (playerId: string, body: string) =>
    apiRequest<ChatMessage>(`/chats/conversations/${playerId}/messages`, {
      method: 'POST',
      body: JSON.stringify({ body }),
    }),
  reports: () => apiRequest<ChatReport[]>('/chats/reports?status=open'),
  resolveReport: (id: string) => apiRequest<void>(`/chats/reports/${id}/resolve`, { method: 'POST' }),
  read: (playerId: string) =>
    apiRequest<{ read_at: string | null }>(`/chats/conversations/${playerId}/read`, { method: 'POST' }),
}
