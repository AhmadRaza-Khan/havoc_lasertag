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

/** One player's record in one game, with everything read from the report. */
export type GameRecord = {
  game: { id: string; started_at: string; ended_at: string; scenario: string | null }
  number_in_game: number
  team: string | null
  team_score: number | null
  points: number
  efficiency: number
  efficiency_kdr: number | null
  frags: number
  deactivated: number
  shots: number
  hits: number
  accuracy: number
  captures: number | null
  hits_taken: number | null
  max_combo: number | null
  place_in_team: number | null
  place_in_game: number | null
  nominations: string[]
  victims: { name: string; count: number | null }[]
  insulters: { name: string; count: number | null }[]
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
