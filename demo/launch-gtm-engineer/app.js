/* GTM Engineer launch microsite — data binding + interactions.
 *
 * Reads data/demo.json, fills every element with [data-bind], and
 * wires the "Approve skill" button to the lifecycle-badge animation.
 * No external dependencies; runs from `file://` (the JSON fetch is
 * the only IO and degrades gracefully when blocked).
 */

(() => {
  "use strict";

  // ---------------------------------------------------- formatters
  const fmt = {
    pct(n) {
      const v = Number(n);
      if (!Number.isFinite(v)) return String(n ?? "");
      return `${(v * 100).toFixed(0)}%`;
    },
    pct1(n) {
      const v = Number(n);
      if (!Number.isFinite(v)) return String(n ?? "");
      return `${(v * 100).toFixed(1)}%`;
    },
    "short-id"(s) {
      if (typeof s !== "string" || s.length < 12) return s ?? "";
      return s.slice(0, 8) + "…" + s.slice(-4);
    },
    date(s) {
      if (!s) return "";
      const d = new Date(s);
      if (Number.isNaN(d.getTime())) return s;
      return d.toISOString().slice(0, 10);
    },
    "memory-summary"(arr) {
      if (!Array.isArray(arr)) return "";
      const counts = {};
      for (const w of arr) counts[w.kind] = (counts[w.kind] || 0) + 1;
      return Object.entries(counts)
        .map(([k, n]) => `${n}× ${k}`)
        .join(" · ");
    },
  };

  // -------------------------------------- nested-key reader (a.b.c)
  function pluck(obj, path) {
    return path.split(".").reduce(
      (acc, k) => (acc == null ? acc : acc[k]),
      obj
    );
  }

  // -------------------------------------- fill all [data-bind] nodes
  function applyBindings(root, data) {
    root.querySelectorAll("[data-bind]").forEach((el) => {
      const key = el.getAttribute("data-bind");
      const format = el.getAttribute("data-format");
      let val = pluck(data, key);
      if (val == null) return;
      if (format && fmt[format]) val = fmt[format](val);
      el.textContent = String(val);
    });

    root.querySelectorAll("[data-bind-list]").forEach((el) => {
      const key = el.getAttribute("data-bind-list");
      const tplSel = el.getAttribute("data-list-template") || "li";
      const items = pluck(data, key);
      if (!Array.isArray(items)) return;
      el.innerHTML = "";
      items.forEach((item) => {
        const li = document.createElement(tplSel.split(".")[0]);
        const className = tplSel.split(".").slice(1).join(" ");
        if (className) li.className = className;
        li.textContent = String(item);
        el.appendChild(li);
      });
    });
  }

  // ----- Approve button → spinner → checkmark + lifecycle pulse
  function wireApprove(badge, btn) {
    if (!badge || !btn) return;
    btn.addEventListener("click", () => {
      btn.classList.add("is-loading");
      // Simulated round-trip — keeps the loop visible for a video
      // without faking any backend. The actual approve happens in
      // the demo script (`argo skill review approve`); this is the
      // story-mode visual.
      setTimeout(() => {
        btn.classList.remove("is-loading");
        btn.classList.add("is-done");
        const labelEl = btn.querySelector(".approve-label");
        if (labelEl) labelEl.textContent = "Active";
        badge.setAttribute("data-state", "active");
        badge.textContent = "active";
        // After the pulse settles, scroll the After section into view
        // so the camera (or the user) flows naturally to it.
        setTimeout(() => {
          const target = document.getElementById("after");
          if (target) target.scrollIntoView({ behavior: "smooth", block: "start" });
        }, 700);
      }, 900);
    });
  }

  // -------- Update OpenGraph meta from JSON for share previews
  function applyOpenGraph(data) {
    const set = (sel, value) => {
      const el = document.querySelector(sel);
      if (el && value) el.setAttribute("content", String(value));
    };
    set('meta[property="og:title"]', data?.story?.headline);
    set('meta[property="og:description"]', data?.story?.subheadline);
    set('meta[name="twitter:card"]', "summary_large_image");
    if (data?.story?.headline) document.title = data.story.headline;
  }

  // ------------------------------------------ degrade gracefully
  function showLoadError(detail) {
    const hero = document.querySelector(".hero .container");
    if (!hero) return;
    const note = document.createElement("p");
    note.className = "hero-meta";
    note.style.color = "var(--weak)";
    note.textContent =
      "Could not load data/demo.json (" + detail + "). " +
      "Run demo/launch-gtm-engineer/capture_demo.sh to generate it, " +
      "or serve this directory over HTTP if your browser blocks " +
      "file:// fetches.";
    hero.appendChild(note);
  }

  // ============================================================ boot
  async function boot() {
    let data;
    try {
      const res = await fetch("data/demo.json", { cache: "no-store" });
      if (!res.ok) throw new Error("HTTP " + res.status);
      data = await res.json();
    } catch (err) {
      // The HTML ships with sensible fallback text so the page is
      // never blank — we just surface the failure for the operator.
      showLoadError(String(err && err.message ? err.message : err));
      return;
    }

    applyBindings(document, data);
    applyOpenGraph(data);

    wireApprove(
      document.getElementById("lifecycle-badge"),
      document.getElementById("approve-btn"),
    );
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
