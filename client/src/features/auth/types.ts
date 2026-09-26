export type User = {
  id: string
  username: string
  name: string
  created_at: string
}

export type LoginResponse = {
  user: User
}
