import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { settingsApi } from './api'
import type { PaymentSettingsInput, SkipCashCredentials, SkipCashCredentialsInput } from './types'

const paymentKey = ['settings', 'payments'] as const
const skipcashKey = ['settings', 'skipcash'] as const

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

export function useSkipCashCredentials() {
  return useQuery({ queryKey: skipcashKey, queryFn: settingsApi.skipcash })
}

/** Saving or removing credentials turns online payment on or off. */
function useCredentialsMutation<T>(fn: (input: T) => Promise<SkipCashCredentials>) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: fn,
    onSuccess: (saved) => {
      qc.setQueryData(skipcashKey, saved)
      void qc.invalidateQueries({ queryKey: paymentKey })
    },
  })
}

export function useSaveSkipCash() {
  return useCredentialsMutation((input: SkipCashCredentialsInput) => settingsApi.saveSkipcash(input))
}

export function useRemoveSkipCash() {
  return useCredentialsMutation(() => settingsApi.removeSkipcash())
}

export function useTestSkipCash() {
  return useMutation({ mutationFn: (input: SkipCashCredentialsInput) => settingsApi.testSkipcash(input) })
}
