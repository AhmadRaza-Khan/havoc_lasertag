import { useRef, useState } from "react";
import type { ChangeEvent } from "react";
import { ImageOff, ImagePlus, RefreshCw, Trash2 } from "lucide-react";
import { ConfirmDialog } from "@/components/ui/ConfirmDialog";
import { prepareImage } from "@/features/offers";
import { apiUrl } from "@/lib/api-client";
import { useAddons, useRemoveAddonImage, useUploadAddonImage } from "../hooks";
import type { Addon } from "../types";

/**
 * A picture for each add-on (Bookeo has none), shown next to the add-on when booking. One
 * picture covers the add-on on every game. Prices stay Bookeo's.
 */
export function AddonImagesSettings() {
  const { data, isPending, isError, error } = useAddons();
  const [toRemove, setToRemove] = useState<Addon | null>(null);
  const remove = useRemoveAddonImage();

  return (
    <section className="settings-section">
      <h2>Add-on pictures</h2>
      <p className="muted">
        Shown next to each add-on when booking. Add-ons and their prices come
        from Bookeo; one picture covers the add-on on every game that offers it.
      </p>

      {isPending ? (
        <p className="muted">Loading add-ons…</p>
      ) : isError ? (
        <p className="error">{error.message}</p>
      ) : data.length === 0 ? (
        <p className="muted">No add-ons in Bookeo yet.</p>
      ) : (
        <ul className="addon-images">
          {data.map((a) => (
            <AddonRow key={a.key} addon={a} onRemove={() => setToRemove(a)} />
          ))}
        </ul>
      )}

      <ConfirmDialog
        open={toRemove !== null}
        title="Remove picture"
        message={
          <>
            Remove the picture of <strong>{toRemove?.name}</strong>? The add-on
            stays; it's just shown without a picture.
          </>
        }
        confirmLabel="Remove"
        pending={remove.isPending}
        error={remove.error?.message}
        onConfirm={() =>
          toRemove &&
          remove.mutate(toRemove.key, { onSuccess: () => setToRemove(null) })
        }
        onClose={() => {
          setToRemove(null);
          remove.reset();
        }}
      />
    </section>
  );
}

function AddonRow({ addon, onRemove }: { addon: Addon; onRemove: () => void }) {
  const fileInput = useRef<HTMLInputElement>(null);
  const upload = useUploadAddonImage();
  const [preparing, setPreparing] = useState(false);
  const [prepError, setPrepError] = useState<string | null>(null);
  const busy = preparing || upload.isPending;

  async function pick(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = ""; // picking the same file again still fires
    if (!file) return;
    setPrepError(null);
    upload.reset();
    setPreparing(true);
    try {
      const image = await prepareImage(file);
      upload.mutate({ key: addon.key, image });
    } catch (err) {
      setPrepError(
        err instanceof Error ? err.message : "Could not read the image.",
      );
    } finally {
      setPreparing(false);
    }
  }

  const error = prepError ?? upload.error?.message;
  return (
    <li className="addon-image-row">
      <div className="addon-image-frame">
        {addon.image_url ? (
          <img src={apiUrl(addon.image_url)} alt="" />
        ) : (
          <ImageOff size={22} aria-hidden="true" className="muted" />
        )}
      </div>
      <div className="addon-image-info">
        <span className="strong">{addon.name}</span>
        <span className="muted small">
          {addon.games.length === 1
            ? addon.games[0]
            : `${addon.games.length} games`}
          {addon.staff_only && " · staff only"}
        </span>
        {error && <span className="error small">{error}</span>}
      </div>
      <div className="addon-image-actions">
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={busy}
          onClick={() => fileInput.current?.click()}
        >
          {addon.image_url ? (
            <RefreshCw size={14} aria-hidden="true" />
          ) : (
            <ImagePlus size={14} aria-hidden="true" />
          )}
          {busy ? "Uploading…" : addon.image_url ? "Replace" : "Add picture"}
        </button>
        {addon.image_url && (
          <button
            type="button"
            className="btn btn-secondary btn-sm addon-image-remove"
            disabled={busy}
            onClick={onRemove}
            aria-label={`Remove the picture of ${addon.name}`}
          >
            <Trash2 size={14} aria-hidden="true" />
            Remove
          </button>
        )}
        <input
          ref={fileInput}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          hidden
          onChange={pick}
        />
      </div>
    </li>
  );
}
