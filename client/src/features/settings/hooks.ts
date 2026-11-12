import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { gameKeys } from '@/features/games'
import { settingsApi } from './api'
import type {
  Addon,
  PaymentSettingsInput,
  SkipCashCredentials,
  SkipCashCredentialsInput,
  StaffInput,
} from './types'

const paymentKey = ['settings', 'payments'] as const
const skipcashKey = ['settings', 'skipcash'] as const
const addonsKey = ['settings', 'addons'] as const
const staffKey = ['settings', 'staff'] as const

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
  return useCredentialsMutation((input: SkipCashCredentialsInput) =>
    settingsApi.saveSkipcash(input),
  )
}

export function useRemoveSkipCash() {
  return useCredentialsMutation(() => settingsApi.removeSkipcash())
}

export function useTestSkipCash() {
  return useMutation({
    mutationFn: (input: SkipCashCredentialsInput) => settingsApi.testSkipcash(input),
  })
}

export function useAddons() {
  return useQuery({ queryKey: addonsKey, queryFn: settingsApi.addons })
}

/** Uploading or removing a picture: the add-ons list comes back, and booking forms refresh. */
function useAddonMutation<T>(fn: (input: T) => Promise<Addon[]>) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: fn,
    onSuccess: (addons) => {
      qc.setQueryData(addonsKey, addons)
      void qc.invalidateQueries({ queryKey: gameKeys.all })
    },
  })
}

export function useUploadAddonImage() {
  return useAddonMutation(({ key, image }: { key: string; image: Blob }) =>
    settingsApi.uploadAddonImage(key, image),
  )
}

export function useRemoveAddonImage() {
  return useAddonMutation((key: string) => settingsApi.removeAddonImage(key))
}

export function useSetAddonPrice() {
  return useAddonMutation(({ key, amount }: { key: string; amount: string }) =>
    settingsApi.setAddonPrice(key, amount),
  )
}

export function useRemoveAddonPrice() {
  return useAddonMutation((key: string) => settingsApi.removeAddonPrice(key))
}

export function useStaff() {
  return useQuery({ queryKey: staffKey, queryFn: settingsApi.staff })
}

/** Staff changes refresh the list. */
function useStaffMutation<T, R>(fn: (input: T) => Promise<R>) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: fn,
    onSuccess: () => void qc.invalidateQueries({ queryKey: staffKey }),
  })
}

export function useCreateStaff() {
  return useStaffMutation((input: StaffInput) => settingsApi.createStaff(input))
}

export function useUpdateMe() {
  return useStaffMutation((input: { name: string; email: string }) => settingsApi.updateMe(input))
}

export function useChangeMyPassword() {
  return useMutation({
    mutationFn: ({ current, next }: { current: string; next: string }) => settingsApi.changeMyPassword(current, next),
  })
}

export function useUpdateStaff() {
  return useStaffMutation(({ id, input }: { id: string; input: { name: string; email: string } }) =>
    settingsApi.updateStaff(id, input),
  )
}

export function useSetStaffPassword() {
  return useMutation({
    mutationFn: ({ id, password }: { id: string; password: string }) => settingsApi.setStaffPassword(id, password),
  })
}

export function useRemoveStaff() {
  return useStaffMutation((id: string) => settingsApi.removeStaff(id))
}
