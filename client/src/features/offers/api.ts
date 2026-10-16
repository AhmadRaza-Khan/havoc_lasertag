import { apiRequest, toQueryString } from '@/lib/api-client'
import type { Paginated } from '@/lib/api-client'
import type { Offer, OfferInput, OfferListParams } from './types'

export const offersApi = {
  list: (params: OfferListParams) => apiRequest<Paginated<Offer>>(`/offers${toQueryString(params)}`),
  create: (input: OfferInput) =>
    apiRequest<Offer>('/offers', { method: 'POST', body: JSON.stringify(input) }),
  update: (id: string, input: OfferInput) =>
    apiRequest<Offer>(`/offers/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
  delete: (id: string) => apiRequest<void>(`/offers/${id}`, { method: 'DELETE' }),
  uploadImage: (id: string, image: Blob) => {
    const body = new FormData()
    body.append('file', image, 'offer.jpg')
    return apiRequest<Offer>(`/offers/${id}/image`, { method: 'PUT', body })
  },
  removeImage: (id: string) => apiRequest<Offer>(`/offers/${id}/image`, { method: 'DELETE' }),
}
