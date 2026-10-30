import { apiRequest } from "@/lib/api-client";
import type {
  Addon,
  PaymentSettings,
  PaymentSettingsInput,
  SkipCashCredentials,
  SkipCashCredentialsInput,
  SkipCashTestResult,
} from "./types";

export const settingsApi = {
  payments: () => apiRequest<PaymentSettings>("/settings/payments"),
  savePayments: (input: PaymentSettingsInput) =>
    apiRequest<PaymentSettings>("/settings/payments", {
      method: "PUT",
      body: JSON.stringify(input),
    }),
  skipcash: () => apiRequest<SkipCashCredentials>("/settings/skipcash"),
  saveSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashCredentials>("/settings/skipcash", {
      method: "PUT",
      body: JSON.stringify(input),
    }),
  removeSkipcash: () =>
    apiRequest<SkipCashCredentials>("/settings/skipcash", { method: "DELETE" }),
  addons: () => apiRequest<Addon[]>("/settings/addons"),
  uploadAddonImage: (key: string, image: Blob) => {
    const body = new FormData();
    body.append("file", image, "addon.jpg");
    return apiRequest<Addon[]>(
      `/settings/addons/${encodeURIComponent(key)}/image`,
      { method: "PUT", body },
    );
  },
  removeAddonImage: (key: string) =>
    apiRequest<Addon[]>(`/settings/addons/${encodeURIComponent(key)}/image`, {
      method: "DELETE",
    }),
  testSkipcash: (input: SkipCashCredentialsInput) =>
    apiRequest<SkipCashTestResult>("/settings/skipcash/test", {
      method: "POST",
      body: JSON.stringify(input),
    }),
};
