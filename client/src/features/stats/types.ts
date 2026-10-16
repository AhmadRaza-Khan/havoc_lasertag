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

/** A name and how often it came up across games (medals, victims, insulters). */
export type Tally = { name: string; count: number }

/** Stats from the players' own report pages, over all their games (`null`: never recorded). */
export type PlayerDetails = {
  /** Total captures. */
  captures: number | null
  /** Total hits taken. */
  hits_taken: number | null
  /** Average "Efficiency KDR" per game. */
  efficiency_kdr: number | null
  /** Best "Max COMBO". */
  max_combo: number | null
  place_in_team: { best: number; average: number } | null
  place_in_game: { best: number; average: number } | null
  /** Medals ("nominations") won, most first. */
  medals: Tally[]
  /** Players they tagged most ("My victims"). */
  victims: Tally[]
  /** Players who tagged them most ("My insulters"). */
  insulters: Tally[]
}

/** One player's totals and personal-page stats (`GET /stats/players/{id}`). */
export type PlayerStats = Standing & { details: PlayerDetails }
