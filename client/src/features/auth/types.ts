export type User = {
  id: string
  username: string
  name: string
  /** For resetting a forgotten password. */
  email: string | null
  /** The owner manages staff accounts. */
  is_owner: boolean
  created_at: string
}

export type LoginResponse = {
  user: User
}
