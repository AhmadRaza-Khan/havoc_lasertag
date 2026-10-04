export type ImportStatus = 'processing' | 'completed' | 'failed'

export type StatImport = {
  id: string
  status: ImportStatus
  source_filename: string
  game: { id: string; started_at: string; ended_at: string } | null
  /** Playernames whose stats were saved. */
  matched_players: string[]
  /** Names in the file with no matching player (kept; linked when that player is created). */
  unmatched_names: string[]
  warnings: string[]
  error: string | null
  created_at: string
  finished_at: string | null
}

/** One row of the general report: a player's all-time totals. */
export type Standing = {
  /** null until the player has played a game */
  position: number | null
  player: { id: string; full_name: string; playername: string }
  games: number
  points: number
  efficiency: number
  frags: number
  deactivated: number
  shots: number
  hits: number
  accuracy: number
  last_played_at: string | null
}
