import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { chatsApi } from './api'

// No socket in the panel: conversations and the open chat are polled.
const CONVERSATIONS_POLL_MS = 5000
const MESSAGES_POLL_MS = 3000

export const chatKeys = {
  all: ['chats'] as const,
  conversations: () => [...chatKeys.all, 'conversations'] as const,
  players: (search: string, page: number) => [...chatKeys.all, 'players', search, page] as const,
  messages: (playerId: string) => [...chatKeys.all, 'messages', playerId] as const,
  reports: () => [...chatKeys.all, 'reports'] as const,
}

/** Open reports from players (blocking/reporting in the app), newest first. */
export function useReports() {
  return useQuery({ queryKey: chatKeys.reports(), queryFn: chatsApi.reports, refetchInterval: 30000 })
}

export function useResolveReport() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => chatsApi.resolveReport(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: chatKeys.reports() }),
  })
}

export function useConversations() {
  return useQuery({
    queryKey: chatKeys.conversations(),
    queryFn: chatsApi.conversations,
    refetchInterval: CONVERSATIONS_POLL_MS,
  })
}

/** Unread messages from players, for the sidebar badge. */
export function useUnreadTotal(): number {
  const { data } = useConversations()
  return data?.reduce((n, c) => n + c.unread_count, 0) ?? 0
}

export function useChatPlayers(search: string, page: number, enabled: boolean) {
  return useQuery({
    queryKey: chatKeys.players(search, page),
    queryFn: () => chatsApi.players(search, page),
    placeholderData: keepPreviousData,
    refetchInterval: enabled ? 15000 : false,
    enabled,
  })
}

/** The latest 50 messages of a chat, newest first (older ones are loaded on demand). */
export function useMessages(playerId: string | null) {
  return useQuery({
    queryKey: chatKeys.messages(playerId ?? ''),
    queryFn: () => chatsApi.messages(playerId!),
    enabled: playerId !== null,
    refetchInterval: MESSAGES_POLL_MS,
  })
}

export function useSendMessage(playerId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: string) => chatsApi.send(playerId, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: chatKeys.messages(playerId) })
      void qc.invalidateQueries({ queryKey: chatKeys.conversations() })
    },
  })
}

export function useMarkRead() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (playerId: string) => chatsApi.read(playerId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: chatKeys.conversations() }),
  })
}
