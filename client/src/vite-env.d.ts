/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL?: string
  /** IANA timezone of the business; booking dates/times are shown in it. */
  readonly VITE_BUSINESS_TIMEZONE?: string
  /** Host of the players' policy site (privacy, terms, account deletion). */
  readonly VITE_PUBLIC_SITE_HOST?: string
}
