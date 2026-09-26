/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL?: string
  /** IANA timezone of the business; booking dates/times are shown in it. */
  readonly VITE_BUSINESS_TIMEZONE?: string
}
