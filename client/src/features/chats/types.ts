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
