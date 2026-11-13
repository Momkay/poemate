<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Widget from "$lib/Widget.svelte";
  import { i18n, t, type Key } from "$lib/i18n.svelte";
  import { layout, loadLayout, resetLayout, saveSoon, sizeKey } from "$lib/layout.svelte";

  type Kind = "Normal" | "Notable" | "Keystone" | "Mastery" | "Jewel" | "Root";
  type Snapshot = {
    log: "searching" | "ok" | "not-found" | "error";
    state: { area_name: string; area_level: number; level: number; town: boolean; character: string | null };
    views: {
      route: {
        act: number;
        index: number;
        total: number;
        step: string;
        label: string;
        hints: string[];
        next: string[];
        rewards: { gem: string; how: "Take" | "Buy"; npc: string }[];
        passive_points: number;
      } | null;
      plan: {
        stage: number;
        stage_title: string;
        stages: string[];
        manual: boolean;
        auto_order: boolean;
        passives_total: number;
        passives_available: number;
        budget: number | null;
        spent_before: number;
        tree: { x: number; y: number; from: [number, number] | null; kind: Kind; state: "done" | "now" | "later" }[];
        passives: { name: string; kind: Kind; stats: string[]; known: boolean; available: boolean }[];
        masteries: [string, string[]][];
        new_gems: string[];
        gems: { name: string; support: boolean }[][];
        gear: { slot: string; name: string; base: string; changed: boolean }[];
        notes: string;
      } | null;
    };
  };

  const GLYPH: Record<Kind, string> = { Normal: "·", Notable: "◆", Keystone: "★", Mastery: "◇", Jewel: "●", Root: "·" };

  let edit = $state(false);
  let poeFound = $state(true);
  let snap = $state<Snapshot | null>(null);
  let input = $state("");
  let importMsg = $state("");
  let busy = $state(false);
  let panel = $state(false);

  const route = $derived(snap?.views.route ?? null);
  const plan = $derived(snap?.views.plan ?? null);
  const kindCounts = $derived(
    (["Notable", "Keystone", "Mastery", "Jewel"] as Kind[])
      .map((k) => [k, plan?.passives.filter((p) => p.kind === k).length ?? 0] as const)
      .filter(([, n]) => n > 0),
  );
  // crop of the tree around what is taken next, keeping the panel's 560:240 shape
  const crop = $derived.by(() => {
    const dots = plan?.tree ?? [];
    const focus = dots.filter((d) => d.state === "now");
    const pts = (focus.length ? focus : dots.filter((d) => d.state === "later")).flatMap((d) => (d.from ? [[d.x, d.y], d.from] : [[d.x, d.y]]));
    if (!pts.length) return null;
    const [x0, x1] = [Math.min(...pts.map((q) => q[0])), Math.max(...pts.map((q) => q[0]))];
    const [y0, y1] = [Math.min(...pts.map((q) => q[1])), Math.max(...pts.map((q) => q[1]))];
    let w = Math.max(x1 - x0 + 700, 1100);
    let h = Math.max(y1 - y0 + 700, w * 0.43);
    w = Math.max(w, h / 0.43);
    return { vb: `${(x0 + x1) / 2 - w / 2} ${(y0 + y1) / 2 - h / 2} ${w} ${h}`, w, next: focus[0] };
  });
  // link groups with the active skill first, supports after it in PoB order
  const links = $derived(
    (plan?.gems ?? [])
      .map((g) => [...g.filter((x) => !x.support), ...g.filter((x) => x.support)])
      .filter((g) => g.length),
  );
  // gems and gear are shopping lists: out in a zone they are clutter
  const shop = $derived((snap?.state.town ?? false) || panel || edit);
  const COMPACT = 5;
  const changedGear =$derived(plan?.gear.filter((g) => g.changed) ?? []);

  // level-up toasts: max 2 at once, 6 s each; the first level seen after start is not a level-up
  let toasts = $state<{ id: number; text: string }[]>([]);
  let lastLevel = 0;
  $effect(() => {
    const lv = snap?.state.level ?? 0;
    if (lastLevel && lv > lastLevel) {
      const id = Date.now();
      const pts = plan?.budget != null ? Math.max(0, plan.budget - plan.spent_before) : 0;
      toasts = [...toasts, { id, text: pts ? t("levelToastPoints", { n: lv, p: pts }) : t("levelToast", { n: lv }) }].slice(-2);
      setTimeout(() => (toasts = toasts.filter((t) => t.id !== id)), 6000);
    }
    if (lv) lastLevel = lv;
  });

  type Binding = { id: string; label: string; keys: string };
  let binds = $state<Binding[]>([]);
  let capturing = $state("");
  let bindMsg = $state("");
  const pretty = (k: string) => k.replace(/Key|Digit|Arrow/g, "");
  const keyOf = (id: string) => pretty(binds.find((b) => b.id === id)?.keys ?? "");

  // turns a key press into the text the backend parses, e.g. "Ctrl+Shift+KeyO"
  async function capture(e: KeyboardEvent, id: string) {
    e.preventDefault();
    if (e.key === "Escape") return void (capturing = "");
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    const keys = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.code].filter(Boolean).join("+");
    try {
      binds = await invoke<Binding[]>("hotkeys_set", { id, keys });
      bindMsg = "";
    } catch (err) {
      bindMsg = String(err);
    }
    capturing = "";
  }

  let ready = $state(false);
  $effect(() => void (i18n.lang = layout.settings.lang));
  const wizard = $derived(ready && !layout.settings.onboarded);
  const setLang = (l: "en" | "nl") => {
    layout.settings.lang = l;
    saveSoon();
  };
  const finish = () => {
    layout.settings.onboarded = true;
    saveSoon();
  };

  let copied = $state("");
  // write-only and only on a user click, so it stays within the GGG overlay rules
  async function copySearch(g: { slot: string; name: string; base: string }) {
    try {
      await navigator.clipboard.writeText(`"${g.name || g.base}"`);
      copied = g.slot;
      setTimeout(() => (copied = ""), 1200);
    } catch {}
  }

  async function doImport() {
    if (!input.trim() || busy) return;
    busy = true;
    importMsg = t("importing");
    try {
      const s = await invoke<{ class: string; ascendancy: string; stages: number }>("import_build", { input });
      importMsg = t("loaded", { name: s.ascendancy || s.class, n: s.stages });
      input = "";
    } catch (e) {
      importMsg = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    loadLayout().then(() => (ready = true));
    invoke<Binding[]>("hotkeys_get").then((b) => (binds = b));
    const onResize = () => (layout.key = sizeKey());
    window.addEventListener("resize", onResize);
    invoke<Snapshot>("snapshot").then((s) => (snap ??= s));
    const un = [
      listen<boolean>("mode", (e) => (edit = e.payload)),
      listen("passive-panel", () => (panel = !panel)),
      listen<boolean>("poe-found", (e) => (poeFound = e.payload)),
      listen<Snapshot>("snapshot", (e) => (snap = e.payload)),
    ];
    return () => {
      window.removeEventListener("resize", onResize);
      un.forEach((p) => void p.then((f) => f()));
    };
  });
</script>

<main class:edit class:compact={layout.settings.density === "compact"} style:--alpha={layout.settings.opacity}>
  <div class="hud">
    {#if snap && snap.log !== "ok"}
      <span class="status">
        {snap.log === "searching" ? t("logSearching") : snap.log === "not-found" ? t("logNotFound") : t("logError")}
      </span>
    {:else}
      <span class="lvl">{snap?.state.level || "–"}</span>
      <span class="sep"></span>
      <span class="area">{snap?.state.area_name || t("waitingZone")}</span>
      {#if snap?.state.area_level}
        <span class="sep"></span>
        <span class="alvl">{t("areaLevel", { n: snap.state.area_level })}</span>
      {/if}
    {/if}
  </div>

  <div class="toasts" aria-live="polite">
    {#each toasts as t (t.id)}<p class="toast">{t.text}</p>{/each}
  </div>

  <div class="col">
    {#if route}
      <Widget id="route" {edit}>
      <section class="card">
        <header><span class="eyebrow">{t("act", { n: route.act })}</span><span class="meta">{route.index + 1} / {route.total}</span></header>
        <div class="bar"><i style:width="{((route.index + 1) / route.total) * 100}%"></i></div>
        <div class="path">
        {#key route.index}<h1 class="step">{route.step}</h1>{/key}
        {#if snap?.state.town && route.rewards.length}
          <span class="chip town">{t("townTasks", { n: route.rewards.length })}</span>
        {/if}
        {#if route.label}<p class="label">{route.label}</p>{/if}
        {#each route.hints as h}<p class="hint-line">{h}</p>{/each}
        {#if route.passive_points > 0}
          <p class="points">{t(route.passive_points > 1 ? "passivePoints" : "passivePoint", { n: route.passive_points })}</p>
        {/if}
        {#each route.rewards as r}
          <p class="reward">
            <span class="mark {r.how === 'Take' ? 'take' : 'buy'}" aria-hidden="true">{r.how === "Take" ? "◆" : "◇"}</span>
            <span class="rtxt"><b>{r.gem}</b><span>{r.how === "Take" ? t("take") : t("buy")} · {r.npc}</span></span>
          </p>
        {/each}
        {#each route.next as n, i}<p class="next hop" style:opacity={1 - i * 0.25}>{n}</p>{/each}
        </div>
      </section>
      </Widget>
    {/if}

    {#if plan}
      <Widget id="passive" {edit}>
      <section class="card" class:panel>
        <header>
          <span class="eyebrow">{plan.stage_title}{plan.manual ? ` · ${t("manual")}` : ""}</span>
          <span class="meta">{t("nowBadge", { a: plan.passives_available, b: plan.passives_total })}</span>
        </header>
        {#if panel}
          <p class="counts">
            {#each kindCounts as [k, n]}<span class={k.toLowerCase()}>{GLYPH[k]} {n}</span>{/each}
          </p>
          <p class="stages">
            {#each plan.stages as s, i}<span class:cur={i === plan.stage}>{s}</span>{/each}
          </p>
        {/if}
        {#if panel && crop && plan}
          <svg class="mini" viewBox={crop.vb} role="img" aria-label="Passive tree around the next points to take">
            {#each plan.tree as d}
              {#if d.from}<line x1={d.from[0]} y1={d.from[1]} x2={d.x} y2={d.y} class="l {d.state}" />{/if}
            {/each}
            {#each plan.tree as d}
              <circle cx={d.x} cy={d.y} r={d.kind === "Keystone" ? 36 : d.kind === "Normal" ? 17 : 27} class="n {d.state} {d.kind.toLowerCase()}" />
            {/each}
            {#if crop.next}<circle cx={crop.next.x} cy={crop.next.y} r="52" class="ring" />{/if}
          </svg>
        {/if}
        {#if plan.budget !== null}
          <p class="next">{t("pointsToSpend", { n: Math.max(0, plan.budget - plan.spent_before) })}</p>
        {/if}
        {#if plan.passives.length === 0}
          <p class="next">{t("noPassives")}</p>
        {/if}
        <ol class="list">
          {#each panel ? plan.passives : plan.passives.slice(0, COMPACT) as p}
            <li class={p.kind.toLowerCase()} class:locked={!p.available}>
              <span class="glyph">{GLYPH[p.kind]}</span>
              <span class="name">{p.name}</span>
              {#if p.kind !== "Normal" && p.stats[0]}<span class="stat">{p.stats[0]}</span>{/if}
            </li>
          {/each}
        </ol>
        {#if !panel && plan.passives_total > COMPACT}<p class="next">{t("moreHint", { n: plan.passives_total - COMPACT, key: keyOf("panel") })}</p>{/if}
        {#each plan.masteries as [name, stats]}
          <p class="mastery">◇ {name}: {stats.join(" · ")}</p>
        {/each}
        {#if plan.auto_order}<p class="hint">{t("autoOrder")}</p>{/if}
      </section>
      </Widget>

      {#if links.length && shop}
        <Widget id="gems" {edit}>
        <section class="card">
          <header>
            <span class="eyebrow">{t("gems")}</span>
            {#if plan.new_gems.length}<span class="meta gain">{t("newGems", { n: plan.new_gems.length })}</span>{/if}
          </header>
          {#each links.slice(0, panel ? 12 : 5) as grp}
            <div class="chips">
              {#each grp as g}
                <span class="chip" class:support={g.support} class:new={plan.new_gems.includes(g.name)}>{g.name}</span>
              {/each}
            </div>
          {/each}
          {#if links.length > (panel ? 12 : 5)}<p class="next">{t("moreLinks", { n: links.length - (panel ? 12 : 5) })}</p>{/if}
        </section>
        </Widget>
      {/if}

      {#if changedGear.length && shop}
        <Widget id="gear" {edit}>
        <section class="card">
          <header><span class="eyebrow">{t("gear")}</span></header>
          <ul class="list">
            {#each changedGear.slice(0, 6) as g}
              <li class:copied={copied === g.slot}>
                <button class="copy" title={t("copyTitle")} onclick={() => copySearch(g)}>
                  <span class="slot">{g.slot}</span><span class="name">{g.name}{g.base ? ` · ${g.base}` : ""}</span>
                </button>
              </li>
            {/each}
          </ul>
          <p class="hint">{t("copyHint")}</p>
        </section>
        </Widget>
      {/if}

      {#if plan.notes.trim() && panel}
        <Widget id="notes" {edit}>
        <section class="card notes">
          <header><span class="eyebrow">{t("notes")}</span></header>
          <p class="notes-body">{plan.notes}</p>
        </section>
        </Widget>
      {/if}
    {:else if snap && !route}
      <section class="card"><p class="next">{t("loadBuild", { key: keyOf("edit") })}</p></section>
    {/if}

    {#if wizard}
      <section class="card">
        <header><span class="eyebrow">{t("welcome")}</span></header>
        {#if edit}
          <p class="langs">
            <span class="eyebrow">{t("language")}</span>
            {#each ["en", "nl"] as const as l}<button class:on={layout.settings.lang === l} onclick={() => setLang(l)}>{l.toUpperCase()}</button>{/each}
          </p>
          <ul class="list">
            <li class:locked={snap?.log !== "ok"}>{snap?.log === "ok" ? "✓" : "○"} {t("wizLog")}</li>
            <li class:locked={!poeFound}>{poeFound ? "✓" : "○"} {t("wizWindow")}</li>
            <li class:locked={!plan}>{plan ? "✓" : "○"} {t("wizBuild")}</li>
          </ul>
          <div class="row"><button onclick={finish}>{t("wizDone")}</button></div>
        {:else}
          <p class="next">{t("wizHint", { key: keyOf("edit") })}</p>
        {/if}
      </section>
    {/if}

    {#if edit}
      <section class="card editbox">
        <header><span class="eyebrow">{t("importTitle")}</span></header>
        <input bind:value={input} placeholder={t("importPlaceholder")} onkeydown={(e) => e.key === "Enter" && doImport()} />
        <div class="row">
          <button onclick={doImport} disabled={busy}>{t("importBtn")}</button>
          <button onclick={() => invoke("route_step", { delta: -1 })}>{t("stepPrev")}</button>
          <button onclick={() => invoke("route_step", { delta: 1 })}>{t("stepNext")}</button>
        </div>
        {#if importMsg}<p class="next">{importMsg}</p>{/if}
        <label class="slider">{t("opacity")}
          <input type="range" min="0.2" max="1" step="0.05" bind:value={layout.settings.opacity} oninput={saveSoon} />
        </label>
        <div class="row">
          <button onclick={() => { layout.settings.density = layout.settings.density === "cozy" ? "compact" : "cozy"; saveSoon(); }}>
            {layout.settings.density === "cozy" ? t("compact") : t("cozy")}
          </button>
          <button onclick={resetLayout}>{t("resetLayout")}</button>
        </div>
        <p class="hint">{t("dragHint")}</p>
        <p class="langs">
          <span class="eyebrow">{t("language")}</span>
          {#each ["en", "nl"] as const as l}<button class:on={layout.settings.lang === l} onclick={() => setLang(l)}>{l.toUpperCase()}</button>{/each}
        </p>
        <details>
          <summary class="eyebrow">{t("hotkeys")}</summary>
          <ul class="list">
            {#each binds as b}
              <li>
                <span class="slot">{t(`act.${b.id}` as Key)}</span>
                <button class="key" onclick={() => (capturing = b.id)} onkeydown={(e) => capturing === b.id && capture(e, b.id)} onblur={() => (capturing = "")}>
                  {capturing === b.id ? t("pressKeys") : pretty(b.keys)}
                </button>
              </li>
            {/each}
          </ul>
          {#if bindMsg}<p class="next warn">{bindMsg}</p>{/if}
          <p class="hint">{t("hotkeyHint")}</p>
        </details>
        <p class="hint">{t("disclaimer")}</p>
      </section>
    {:else}
      <p class="foot">{t("footer", { edit: keyOf("edit"), hide: keyOf("hide") })}</p>
    {/if}
    {#if !poeFound}
      <p class="foot warn">{t("noPoe")}</p>
    {/if}
  </div>
</main>

<style>
  /* Waymarker look: warm ink surfaces stepped by tone, one ember accent that only marks "where you are now",
     square-cut shapes and diamond waymarks borrowed from the passive tree; no blur, no pills, no avatars */
  :global(:root) {
    --bg-0: 14, 12, 10;
    --bg-1: 25, 22, 19;
    --bg-2: 40, 35, 30;
    --hover: #3a342d;
    --text: #d9d2c5;
    --strong: #f4efe4;
    --muted: #9a9082;
    --accent: #e2813b;
    --green: #5fae8f;
    --yellow: #d9a93c;
    --red: #d65a3a;
  }
  :global(html, body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
    font-family: "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
    font-size: 14px;
    color: var(--text);
    user-select: none;
  }
  main {
    position: fixed;
    inset: 0;
    padding: 18px;
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition: outline-color 0.15s ease-out;
  }
  main.edit {
    outline-color: rgba(88, 101, 242, 0.7);
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 300px;
    margin-top: 56px;
  }
  .hud,
  .card {
    background: rgba(var(--bg-1), var(--alpha, 0.92));
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  }
  .hud {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px 4px 4px;
    border-radius: 4px;
    border-bottom: 2px solid var(--accent);
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    color: var(--strong);
  }
  .lvl {
    display: grid;
    place-items: center;
    min-width: 24px;
    height: 24px;
    padding: 0 4px;
    box-sizing: border-box;
    font-family: Bahnschrift, "Segoe UI Variable Display", sans-serif;
    font-size: 17px;
    font-weight: 700;
    color: var(--accent);
  }
  .alvl {
    color: var(--muted);
    font-weight: 400;
  }
  .status {
    padding-left: 8px;
    color: var(--yellow);
  }
  .sep {
    display: none;
  }
  .card {
    padding: 12px;
    border-radius: 4px;
    animation: in 0.16s ease-out;
  }
  .compact .card {
    padding: 8px 10px;
  }
  .compact .col {
    gap: 5px;
  }
  .compact h1 {
    font-size: 15px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .eyebrow {
    font-family: Bahnschrift, "Segoe UI Variable Display", sans-serif;
    font-size: 14px;
    font-weight: 600;
    color: var(--muted);
  }
  .meta {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    white-space: nowrap;
  }
  .meta.gain {
    color: var(--green);
  }
  .bar {
    height: 4px;
    margin-top: 8px;
    border-radius: 2px;
    background: rgb(var(--bg-2));
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
    transition: width 0.25s ease-out;
  }
  h1 {
    margin: 10px 0 2px;
    font-size: 17px;
    font-weight: 700;
    line-height: 1.25;
    color: var(--strong);
  }
  p {
    margin: 0;
  }
  .label {
    font-size: 13px;
    color: var(--muted);
  }
  .hint-line {
    margin-top: 4px;
    padding: 5px 8px;
    border-radius: 4px;
    background: rgb(var(--bg-2));
    font-size: 13px;
  }
  .points {
    margin-top: 6px;
    font-size: 13px;
    font-weight: 600;
    color: var(--yellow);
  }
  .locked {
    opacity: 0.55;
  }
  .reward {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    padding: 4px 6px 4px 4px;
    border-radius: 3px;
    background: rgb(var(--bg-2));
  }
  .mark {
    flex: none;
    width: 20px;
    text-align: center;
    font-size: 15px;
  }
  .mark.take {
    color: var(--green);
  }
  .mark.buy {
    color: var(--muted);
  }
  /* the route as a path: the current step is a waymark, what follows hangs below it on the same line */
  .path {
    position: relative;
    margin-top: 10px;
    padding-left: 20px;
  }
  .path::before {
    content: "";
    position: absolute;
    left: 5px;
    top: 14px;
    bottom: 6px;
    width: 2px;
    background: rgb(var(--bg-2));
  }
  .path h1 {
    margin-top: 0;
  }
  .path h1::before,
  .hop::before {
    content: "";
    position: absolute;
    left: 0;
    width: 12px;
    height: 12px;
    transform: rotate(45deg) scale(0.75);
  }
  .path h1::before {
    top: 5px;
    background: var(--accent);
  }
  .hop {
    position: relative;
  }
  .hop::before {
    top: 3px;
    border: 2px solid var(--muted);
    box-sizing: border-box;
    background: rgb(var(--bg-1));
  }
  .rtxt {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .rtxt b {
    font-size: 13px;
    color: var(--strong);
  }
  .rtxt span {
    font-size: 12px;
    color: var(--muted);
  }
  .next {
    margin-top: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .list {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
    font-size: 13px;
  }
  .list li {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    padding: 3px 6px 3px 4px;
    border-radius: 4px;
  }
  .glyph {
    display: grid;
    place-items: center;
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    background: rgb(var(--bg-2));
    font-size: 11px;
    color: var(--muted);
  }
  .notable .glyph {
    background: var(--yellow);
    color: #1e1f22;
  }
  .keystone .glyph {
    background: var(--red);
    color: #fff;
  }
  .mastery .glyph,
  .jewel .glyph {
    background: var(--accent);
    color: #fff;
  }
  .name {
    font-weight: 500;
    color: var(--strong);
  }
  .stat {
    flex-basis: 100%;
    padding-left: 28px;
    margin-top: -3px;
    font-size: 12px;
    color: var(--muted);
  }
  p.mastery {
    margin-top: 8px;
    font-size: 12px;
    color: var(--green);
  }
  .card.panel {
    width: 560px;
  }
  .panel .list {
    grid-template-columns: 1fr 1fr;
    column-gap: 12px;
  }
  .mini {
    display: block;
    width: 100%;
    margin-top: 8px;
    border-radius: 6px;
    background: rgb(var(--bg-0));
  }
  .mini .l {
    stroke: #3a3c41;
    stroke-width: 7;
  }
  .mini .l.now,
  .mini .l.later {
    stroke: #6d6f78;
  }
  .mini .n {
    fill: #4e5058;
  }
  .mini .n.later {
    fill: none;
    stroke: #6d6f78;
    stroke-width: 6;
  }
  .mini .n.now {
    fill: var(--strong);
  }
  .mini .n.now.notable {
    fill: var(--yellow);
  }
  .mini .n.now.keystone {
    fill: var(--red);
  }
  .mini .n.now.mastery,
  .mini .n.now.jewel {
    fill: var(--accent);
  }
  .mini .ring {
    fill: none;
    stroke: var(--strong);
    stroke-width: 7;
  }
  .counts,
  .stages {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
    font-size: 12px;
    font-weight: 600;
  }
  .counts span {
    padding: 1px 8px;
    border-radius: 3px;
    background: rgb(var(--bg-2));
  }
  .counts .notable {
    color: var(--yellow);
  }
  .counts .keystone {
    color: var(--red);
  }
  .stages span {
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 500;
    color: var(--muted);
  }
  .stages .cur {
    background: var(--hover);
    color: var(--strong);
  }
  .slot {
    min-width: 92px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    color: var(--muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .chip {
    padding: 2px 10px;
    border-radius: 3px;
    background: rgb(var(--bg-2));
    font-size: 12px;
    font-weight: 500;
    color: var(--strong);
  }
  .notes-body {
    max-height: 260px;
    margin-top: 8px;
    overflow: hidden;
    font-size: 12px;
    line-height: 1.4;
    white-space: pre-wrap;
    color: var(--text);
  }
  summary {
    margin-top: 10px;
    cursor: pointer;
  }
  .langs {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 10px;
  }
  .langs .eyebrow {
    margin-right: auto;
  }
  .langs button.on {
    background: var(--accent);
  }
  button.key {
    padding: 2px 8px;
    font-size: 12px;
  }
  .step {
    animation: tick 0.45s ease-out;
  }
  .chip.town {
    display: inline-block;
    margin-top: 6px;
    background: var(--yellow);
    color: #1e1f22;
    font-weight: 700;
  }
  @keyframes tick {
    0% {
      opacity: 0;
      transform: translateX(-8px);
      color: var(--green);
    }
    100% {
      opacity: 1;
      transform: none;
    }
  }
  .toasts {
    position: absolute;
    top: 52px;
    left: 50%;
    transform: translateX(-50%);
    display: grid;
    gap: 6px;
    justify-items: center;
  }
  .toast {
    padding: 6px 14px;
    border-radius: 4px;
    background: var(--accent);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    font-size: 13px;
    font-weight: 600;
    color: #1a1612;
    animation: in 0.16s ease-out;
  }
  button.copy {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    padding: 0;
    background: none;
    text-align: left;
  }
  button.copy:hover:not(:disabled) {
    background: var(--hover);
  }
  .list li.copied {
    background: var(--green);
  }
  .chip.support {
    background: transparent;
    box-shadow: inset 0 0 0 1px rgb(var(--bg-2));
    font-weight: 400;
    color: var(--muted);
  }
  .chip.new {
    background: var(--green);
    box-shadow: none;
    color: #fff;
  }
  .hint,
  .foot {
    margin-top: 8px;
    font-size: 11px;
    color: var(--muted);
  }
  .foot {
    padding-left: 4px;
  }
  .warn {
    color: var(--yellow);
  }
  .editbox input:not([type]) {
    width: 100%;
    box-sizing: border-box;
    margin-top: 8px;
    padding: 8px 10px;
    border: 0;
    border-radius: 4px;
    background: rgb(var(--bg-0));
    color: var(--text);
    font: inherit;
    font-size: 13px;
    user-select: text;
  }
  .editbox input:focus {
    outline: 2px solid var(--accent);
  }
  .slider {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 10px;
    font-size: 13px;
  }
  .slider input {
    width: 140px;
    margin: 0;
    padding: 0;
    accent-color: var(--accent);
  }
  .row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  button {
    padding: 6px 12px;
    border: 0;
    border-radius: 3px;
    background: #4e5058;
    color: #fff;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.1s;
  }
  button:hover:not(:disabled) {
    background: #6d6f78;
  }
  .row button:first-child {
    background: var(--accent);
  }
  .row button:first-child:hover:not(:disabled) {
    background: #c4682a;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card,
    .toast,
    .step {
      animation: none;
    }
    .bar i {
      transition: none;
    }
  }
</style>
