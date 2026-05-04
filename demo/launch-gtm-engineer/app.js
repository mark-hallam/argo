/* SKYCOT launch microsite — data binding + interactions.
 *
 * Reads data/demo.json, fills every [data-bind] node, wires the
 * "Approve skill" button to the lifecycle-badge animation, surfaces
 * sections on scroll, and keeps the active nav link in sync with
 * what's in view.
 *
 * No external dependencies; runs from `file://` (the JSON fetch is
 * the only IO and degrades gracefully when blocked).
 */

(() => {
  "use strict";

  // ── formatters ───────────────────────────────────
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
      return Object.entries(counts).map(([k, n]) => `${n}× ${k}`).join(" · ");
    },
  };

  // ── tiny utils ───────────────────────────────────
  const $ = (id) => document.getElementById(id);
  const pluck = (obj, path) => path.split(".").reduce(
    (acc, k) => (acc == null ? acc : acc[k]), obj,
  );

  // ── bind every [data-bind] node ──────────────────
  function applyBindings(root, data) {
    root.querySelectorAll("[data-bind]").forEach((el) => {
      const key = el.getAttribute("data-bind");
      const format = el.getAttribute("data-format");
      let val = pluck(data, key);
      if (val == null) return;
      if (format && fmt[format]) val = fmt[format](val);
      el.textContent = String(val);
    });
  }

  // ── og + title from data ─────────────────────────
  function applyOpenGraph(data) {
    const set = (sel, v) => {
      const el = document.querySelector(sel);
      if (el && v) el.setAttribute("content", String(v));
    };
    // Hero copy is fixed by the launch design, but if a fresh
    // capture wants to push a custom title that can win.
    if (data?.story?.headline) {
      // og only — leave document title (the brand line) alone
      set('meta[property="og:title"]', data.story.headline);
    }
    if (data?.story?.subheadline) {
      set('meta[property="og:description"]', data.story.subheadline);
    }
    set('meta[name="twitter:card"]', "summary_large_image");
  }

  // ── degrade gracefully when fetch blocked ────────
  function showLoadError(detail) {
    const hero = document.querySelector(".hero-inner");
    if (!hero) return;
    const note = document.createElement("p");
    note.className = "body-sm";
    note.style.cssText = "color: var(--warn); margin-top: 16px";
    note.textContent =
      "Could not load data/demo.json (" + detail + "). " +
      "Run demo/launch-gtm-engineer/capture_demo.sh to generate it, " +
      "or serve this folder over HTTP if your browser blocks file:// fetches.";
    hero.appendChild(note);
  }

  // ── Approve button → spinner → lifecycle pulse ──
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
        // After the pulse settles, scroll camera to the After
        // section so the story flows naturally.
        setTimeout(() => {
          const target = $("after");
          if (target) target.scrollIntoView({ behavior: "smooth", block: "start" });
        }, 750);
      }, 900);
    });
  }

  // ── nav active-link based on viewport intersection ──
  function wireNav() {
    const links = document.querySelectorAll(".nav-links a[data-route]");
    if (!links.length || !("IntersectionObserver" in window)) return;
    const setActive = (id) =>
      links.forEach((l) => l.classList.toggle("active", l.dataset.route === id));
    const io = new IntersectionObserver((entries) => {
      const v = entries.filter((e) => e.isIntersecting)
        .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
      if (v) setActive(v.target.getAttribute("data-section"));
    }, { threshold: [0.4] });
    document.querySelectorAll("[data-section]").forEach((s) => io.observe(s));
  }

  // ── scroll-reveal for sections + cards ──────────
  function wireReveals() {
    if (!("IntersectionObserver" in window)) return;
    const targets = document.querySelectorAll(
      ".section .h-1, .flow-card, .card, .diff-list li, .proof-step, .cta-card, .problem-list li"
    );
    targets.forEach((el, i) => {
      el.classList.add("reveal");
      el.style.transitionDelay = `${Math.min(i % 7, 6) * 60}ms`;
    });
    const io = new IntersectionObserver((entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) {
          e.target.classList.add("in");
          io.unobserve(e.target);
        }
      });
    }, { threshold: 0.15, rootMargin: "0px 0px -10% 0px" });
    targets.forEach((el) => io.observe(el));
  }

  // ── video-placeholder click → scroll to #demo ──
  function wireVideoPlaceholder() {
    const btn = document.querySelector(".video-play");
    if (!btn) return;
    btn.addEventListener("click", () => {
      const target = $("demo");
      if (target) target.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }

  // ── boot ────────────────────────────────────────
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
      // Even without data, animations should still run.
      wireNav();
      wireReveals();
      wireVideoPlaceholder();
      wireApprove($("lifecycle-badge"), $("approve-btn"));
      return;
    }

    applyBindings(document, data);
    applyOpenGraph(data);

    wireApprove($("lifecycle-badge"), $("approve-btn"));
    wireNav();
    wireReveals();
    wireVideoPlaceholder();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
