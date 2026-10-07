import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { settingsApi } from './api'
import type { PaymentSettingsInput } from './types'

const paymentKey = ['settings', 'payments'] as const

export function usePaymentSettings() {
  return useQuery({ queryKey: paymentKey, queryFn: settingsApi.payments })
}

export function useSavePaymentSettings() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (input: PaymentSettingsInput) => settingsApi.savePayments(input),
    onSuccess: (saved) => qc.setQueryData(paymentKey, saved),
  })
}
