/** Timezone of the venue. Bookeo slots and bookings are displayed in it, whatever the viewer's timezone. */
export const BUSINESS_TIMEZONE = import.meta.env.VITE_BUSINESS_TIMEZONE ?? 'Asia/Qatar'

/**
 * The players' site for the privacy policy, terms and account deletion. The same build serves it;
 * on this host only those pages exist (no staff sign-in, no admin screens).
 */
export const PUBLIC_SITE_HOST = import.meta.env.VITE_PUBLIC_SITE_HOST ?? 'policy.havoc-tlt.com'

export const isPublicSite = () => window.location.hostname === PUBLIC_SITE_HOST
