import type { GameOption } from "@/features/games";
import { apiUrl } from "@/lib/api-client";
import type { AddOnValues } from "../addOns";

type AddOnsPickerProps = {
  options: GameOption[];
  values: AddOnValues;
  onChange: (id: string, value: string) => void;
};

/** Bookeo's add-ons for the selected game (e.g. "UPGRADE YOUR BATTLE"). */
export function AddOnsPicker({ options, values, onChange }: AddOnsPickerProps) {
  return (
    <div className="addons">
      {options.map((o) => (
        <div
          key={o.id}
          className={o.image_url ? "addon-card has-image" : "addon-card"}
        >
          {o.image_url && (
            <img className="addon-picture" src={apiUrl(o.image_url)} alt="" />
          )}
          <fieldset className="addon">
            <legend className="addon-name">
              {o.name}
              {o.staff_only && (
                <span className="badge addon-staff">Staff only</span>
              )}
            </legend>
            {o.description && (
              <p className="addon-description muted small">{o.description}</p>
            )}

            {o.kind === "choice" && (
              <div
                className="addon-choices"
                role="radiogroup"
                aria-label={o.name}
              >
                {o.values.map((v) => {
                  const checked = values[o.id] === v.id;
                  return (
                    <label
                      key={v.id}
                      className={`addon-choice${checked ? " selected" : ""}`}
                    >
                      <input
                        type="radio"
                        name={o.id}
                        checked={checked}
                        onChange={() => onChange(o.id, v.id)}
                      />
                      <span className="addon-choice-text">
                        <span className="strong">{v.name}</span>
                        {v.description && (
                          <span className="muted small">{v.description}</span>
                        )}
                      </span>
                    </label>
                  );
                })}
                {values[o.id] && (
                  <button
                    type="button"
                    className="btn-link small"
                    onClick={() => onChange(o.id, "")}
                  >
                    Clear choice
                  </button>
                )}
              </div>
            )}

            {o.kind === "on_off" && (
              <label
                className={`addon-toggle${values[o.id] === "true" ? " selected" : ""}`}
              >
                <input
                  type="checkbox"
                  checked={values[o.id] === "true"}
                  onChange={(e) =>
                    onChange(o.id, e.target.checked ? "true" : "false")
                  }
                />
                <span>Add to this booking</span>
                {values[o.id] === "true" && (
                  <span className="addon-toggle-added">✓ Added</span>
                )}
              </label>
            )}

            {o.kind === "number" && (
              <input
                type="number"
                aria-label={o.name}
                className="addon-number"
                min={o.min ?? undefined}
                max={o.max ?? undefined}
                value={values[o.id] ?? ""}
                onChange={(e) => onChange(o.id, e.target.value)}
              />
            )}

            {o.kind === "text" && (
              <input
                aria-label={o.name}
                required={o.required}
                maxLength={500}
                value={values[o.id] ?? ""}
                onChange={(e) => onChange(o.id, e.target.value)}
                placeholder={o.required ? "Required" : "Optional"}
              />
            )}
          </fieldset>
        </div>
      ))}
    </div>
  );
}
