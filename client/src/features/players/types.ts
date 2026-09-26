export type Player = {
  id: string
  full_name: string
  playername: string
  phone: string | null
  email: string | null
  created_at: string
  updated_at: string
}

/** Body for create and update. On update, an empty password keeps the current one. */
export type PlayerInput = {
  full_name: string
  playername: string
  phone: string
  email: string
  password: string
}

export type PlayerListParams = {
  search?: string
  page?: number
  per_page?: number
}
