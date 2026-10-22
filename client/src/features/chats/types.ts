/** A player as seen in chat (`/chats/players`). */
export type ChatPlayer = {
  id: string
  full_name: string
  playername: string
  last_seen_at: string | null
  /** API path of their profile picture, if any. */
  avatar_url: string | null
  /** The app is open right now. */
  online: boolean
}

export type ChatMessage = {
  id: string
  conversation_id: string
  sender_id: string
  recipient_id: string
  body: string
  created_at: string
  /** When the recipient read it. */
  read_at: string | null
}

/** A conversation of Havoc Admin with a player. */
export type Conversation = {
  id: string
  player: ChatPlayer
  last_message: Omit<ChatMessage, 'conversation_id' | 'recipient_id'> | null
  /** Messages from the player not read yet. */
  unread_count: number
}

export type MessagesPage = {
  /** Newest first. */
  items: ChatMessage[]
  has_more: boolean
}

/** A player's report about another player or one of their messages. */
export type ChatReport = {
  id: string
  reporter_id: string | null
  reported_id: string | null
  reporter_playername: string
  reported_playername: string
  message_id: string | null
  /** A copy of the reported message, if a message was reported. */
  message_body: string | null
  reason: 'spam' | 'harassment' | 'inappropriate' | 'other'
  details: string | null
  created_at: string
  resolved_at: string | null
}
