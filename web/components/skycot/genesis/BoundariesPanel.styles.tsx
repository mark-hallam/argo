"use client";

/**
 * Styles for {@link ./BoundariesPanel}. Split out to keep the component file
 * under the 500-LOC ceiling. Rendered as `<style jsx global>` (NOT scoped) so a
 * single style block covers both the editor and the read-only "applied" render
 * — every selector is `.bp`-prefixed to avoid global collisions. Consumes ONLY
 * the locked brand tokens (purple→PINK gradient, Inter, `--brand-ink`) — no
 * color literals, no new design (DEC-027).
 *
 * `"use client"` is required: a `<style jsx>` block in a server component throws
 * (per `feedback_styled_jsx_requires_use_client`).
 */
export function BoundariesPanelStyles() {
  return (
    <style jsx global>{`
      .bp {
        margin: 14px auto 0;
        max-width: 560px;
        font-family: var(--font-sans);
        color: var(--brand-ink);
        display: flex;
        flex-direction: column;
        gap: 14px;
        text-align: left;
        padding: 18px;
        border-radius: 16px;
        border: 1.5px solid color-mix(in srgb, var(--brand-primary) 18%, transparent);
        background: color-mix(in srgb, var(--brand-primary) 3%, #fff);
      }
      .bp-title {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-size: 15px;
        font-weight: 800;
      }
      .bp-icon {
        color: var(--brand-primary);
        flex-shrink: 0;
      }
      .bp-hint {
        color: color-mix(in srgb, var(--brand-ink) 60%, transparent);
        font-size: 12.5px;
        line-height: 1.45;
        margin: 0;
      }
      .bp-section {
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .bp-section-label {
        display: inline-flex;
        align-items: center;
        gap: 7px;
        font-size: 13px;
        font-weight: 700;
      }
      .bp-access-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
      }
      .bp-access-label {
        font-size: 13.5px;
      }
      .bp-seg {
        display: inline-flex;
        border-radius: 999px;
        overflow: hidden;
        border: 1.5px solid color-mix(in srgb, var(--brand-primary) 28%, transparent);
      }
      .bp-seg-btn {
        padding: 6px 12px;
        font-size: 12.5px;
        font-weight: 600;
        border: none;
        background: transparent;
        color: var(--brand-ink);
        cursor: pointer;
      }
      .bp-seg-btn.is-active {
        background: linear-gradient(135deg, var(--brand-primary), var(--brand-pink));
        color: #fff;
      }
      .bp-seg-btn:disabled {
        opacity: 0.55;
        cursor: not-allowed;
      }
      .bp-check {
        display: flex;
        align-items: flex-start;
        gap: 8px;
        font-size: 13px;
        line-height: 1.4;
        cursor: pointer;
      }
      .bp-slider-row {
        display: flex;
        align-items: center;
        gap: 12px;
      }
      .bp-slider-row input[type="range"] {
        flex: 1 1 auto;
        accent-color: var(--brand-primary);
      }
      .bp-amount {
        font-weight: 700;
        font-size: 14px;
        white-space: nowrap;
      }
      .bp-input {
        font-family: inherit;
        font-size: 14px;
        color: var(--brand-ink);
        padding: 9px 12px;
        border-radius: 10px;
        border: 1.5px solid color-mix(in srgb, var(--brand-primary) 28%, transparent);
        background: color-mix(in srgb, var(--brand-primary) 4%, #fff);
      }
      .bp-input:focus {
        outline: none;
        border-color: var(--brand-primary);
      }
      .bp-input:disabled {
        opacity: 0.55;
        cursor: not-allowed;
      }
      .bp-row {
        display: flex;
        align-items: center;
        gap: 8px;
        flex-wrap: wrap;
      }
      .bp-grow {
        flex: 1 1 auto;
        min-width: 0;
      }
      .bp-btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 9px 14px;
        border-radius: 999px;
        border: 1.5px solid color-mix(in srgb, var(--brand-primary) 35%, transparent);
        background: color-mix(in srgb, var(--brand-primary) 6%, #fff);
        color: var(--brand-ink);
        font-size: 13.5px;
        font-weight: 600;
        cursor: pointer;
      }
      .bp-btn:disabled {
        opacity: 0.55;
        cursor: not-allowed;
      }
      .bp-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }
      .bp-chip {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 7px 10px 7px 13px;
        border-radius: 999px;
        background: linear-gradient(
          135deg,
          color-mix(in srgb, var(--brand-primary) 12%, #fff),
          color-mix(in srgb, var(--brand-pink) 12%, #fff)
        );
        border: 1.5px solid color-mix(in srgb, var(--brand-pink) 28%, transparent);
        font-size: 13px;
        font-weight: 600;
        max-width: 100%;
      }
      .bp-chip-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 420px;
      }
      .bp-chip-clear {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 20px;
        height: 20px;
        border-radius: 999px;
        border: none;
        background: color-mix(in srgb, var(--brand-ink) 8%, transparent);
        color: var(--brand-ink);
        cursor: pointer;
        flex-shrink: 0;
      }
    `}</style>
  );
}
