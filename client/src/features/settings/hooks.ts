import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { gameKeys } from "@/features/games";
import { settingsApi } from "./api";
import type {
  Addon,
  PaymentSettingsInput,
  SkipCashCredentials,
  SkipCashCredentialsInput,
} from "./types";

const paymentKey = ["settings", "payments"] as const;
const skipcashKey = ["settings", "skipcash"] as const;
const addonsKey = ["settings", "addons"] as const;

export function usePaymentSettings() {
  return useQuery({ queryKey: paymentKey, queryFn: settingsApi.payments });
}

export function useSavePaymentSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: PaymentSettingsInput) =>
      settingsApi.savePayments(input),
    onSuccess: (saved) => qc.setQueryData(paymentKey, saved),
  });
}

export function useSkipCashCredentials() {
  return useQuery({ queryKey: skipcashKey, queryFn: settingsApi.skipcash });
}

/** Saving or removing credentials turns online payment on or off. */
function useCredentialsMutation<T>(
  fn: (input: T) => Promise<SkipCashCredentials>,
) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (saved) => {
      qc.setQueryData(skipcashKey, saved);
      void qc.invalidateQueries({ queryKey: paymentKey });
    },
  });
}

export function useSaveSkipCash() {
  return useCredentialsMutation((input: SkipCashCredentialsInput) =>
    settingsApi.saveSkipcash(input),
  );
}

export function useRemoveSkipCash() {
  return useCredentialsMutation(() => settingsApi.removeSkipcash());
}

export function useTestSkipCash() {
  return useMutation({
    mutationFn: (input: SkipCashCredentialsInput) =>
      settingsApi.testSkipcash(input),
  });
}

export function useAddons() {
  return useQuery({ queryKey: addonsKey, queryFn: settingsApi.addons });
}

/** Uploading or removing a picture: the add-ons list comes back, and booking forms refresh. */
function useAddonMutation<T>(fn: (input: T) => Promise<Addon[]>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: (addons) => {
      qc.setQueryData(addonsKey, addons);
      void qc.invalidateQueries({ queryKey: gameKeys.all });
    },
  });
}

export function useUploadAddonImage() {
  return useAddonMutation(({ key, image }: { key: string; image: Blob }) =>
    settingsApi.uploadAddonImage(key, image),
  );
}

export function useRemoveAddonImage() {
  return useAddonMutation((key: string) => settingsApi.removeAddonImage(key));
}
