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
  const COMPACT = 3;
  const changedGear =$derived(plan?.gear.filter((g) => g.changed) ?? []);

  // level-up toasts: max 2 at once, 6 s each; the first level seen after start is not a level-up
  let toasts = $state<{ id: number; text: string }[]>([]);
  let lastLevel = 0;
  let lastChar: string | null = null;
  $effect(() => {
    const lv = snap?.state.level ?? 0;
    const who = snap?.state.character ?? null;
    if (who !== lastChar) lastLevel = 0; // another character: its level is a start, not a level-up
    lastChar = who;
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
  const setQuests = (q: "essential" | "all") => {
    layout.settings.quests = q;
    saveSoon();
    invoke("quest_mode", { allQuests: q === "all" });
  };
  const setBandit = (e: Event) => {
    layout.settings.bandit = (e.currentTarget as HTMLSelectElement).value as typeof layout.settings.bandit;
    saveSoon();
    invoke("bandit_choice", { choice: layout.settings.bandit });
  };
  let logPath = $state("");
  let logMsg = $state("");
  async function pickLog() {
    try {
      await invoke("set_log_path", { path: logPath });
      logMsg = t("logPickSaved");
      logPath = "";
    } catch (e) {
      logMsg = String(e);
    }
  }
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
    {#each toasts as toast (toast.id)}<p class="toast">{toast.text}</p>{/each}
  </div>

  <div class="col">
    {#if route}
      <Widget id="route" {edit}>
      <section class="card">
        <p class="where">{t("act", { n: route.act })} · {route.index + 1} / {route.total}</p>
        {#key route.index}<h1 class="step">{route.step}</h1>{/key}
        {#if route.label}<p class="label">{route.label}</p>{/if}
        {#each route.hints.slice(0, 1) as h}<p class="label">{h}</p>{/each}
        {#if route.passive_points > 0}
          <p class="points">{t(route.passive_points > 1 ? "passivePoints" : "passivePoint", { n: route.passive_points })}</p>
        {/if}
        {#each route.rewards as r}
          <p class="reward"><b>{r.gem}</b><span>{r.how === "Take" ? t("take") : t("buy")} · {r.npc}</span></p>
        {/each}
        {#each route.next.slice(0, 1) as n}<p class="next">{n}</p>{/each}
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
        {#if plan.budget !== null && panel}
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
              {#if panel && p.kind !== "Normal" && p.stats[0]}<span class="stat">{p.stats[0]}</span>{/if}
            </li>
          {/each}
        </ol>
        {#if !panel && plan.passives_total > COMPACT}<p class="next">{t("moreHint", { n: plan.passives_total - COMPACT, key: keyOf("panel") })}</p>{/if}
        {#each panel ? plan.masteries : [] as [name, stats]}
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
          {#each links.slice(0, panel ? 12 : 4) as grp}
            <p class="grp">
              {#each grp as g, i}<span class:support={g.support} class:new={plan.new_gems.includes(g.name)}>{g.name}</span>{i < grp.length - 1 ? " · " : ""}{/each}
            </p>
          {/each}
          {#if links.length > (panel ? 12 : 4)}<p class="next">{t("moreLinks", { n: links.length - (panel ? 12 : 4) })}</p>{/if}
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

  </div>

  <!-- settings sit bottom right: the top left is where people keep their own panels -->
  <aside class="settings">
    {#if snap?.log === "not-found"}
      <section class="card editbox">
        <header><span class="eyebrow">{t("logPickTitle")}</span></header>
        {#if edit}
          <input bind:value={logPath} placeholder={t("logPickPlaceholder")} onkeydown={(e) => e.key === "Enter" && pickLog()} />
          <div class="row"><button onclick={pickLog}>{t("logPickBtn")}</button></div>
          {#if logMsg}<p class="next">{logMsg}</p>{/if}
          <p class="hint">{t("logPickHint")}</p>
        {:else}
          <p class="next">{t("wizHint", { key: keyOf("edit") })}</p>
        {/if}
      </section>
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
          <span class="eyebrow">{t("questsTitle")}</span>
          {#each ["essential", "all"] as const as q}
            <button class:on={layout.settings.quests === q} onclick={() => setQuests(q)}>{q === "all" ? t("questsAll") : t("questsEssential")}</button>
          {/each}
        </p>
        <p class="hint">{t("questsHint")}</p>
        <label class="slider">{t("banditTitle")}
          <select value={layout.settings.bandit} onchange={setBandit}>
            {#each ["build", "kill", "alira", "oak", "kraityn"] as b}
              <option value={b}>{t(`bandit${b[0].toUpperCase()}${b.slice(1)}` as Key)}</option>
            {/each}
          </select>
        </label>
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
    {/if}
    {#if !edit}
      <p class="foot">{t("footer", { edit: keyOf("edit"), hide: keyOf("hide") })}</p>
    {/if}
    {#if !poeFound}
      <p class="foot warn">{t("noPoe")}</p>
    {/if}
  </aside>
</main>

<style>
  /* Chart-room look: blue-black ink stepped by tone, one sea-glass accent for "now" and "gain", a serif for titles
     (the game's own face is a serif), soft corners, arrows for the route ahead; no blur, no pills, no avatars */
  :global(:root) {
    --bg-0: 9, 14, 17;
    --bg-1: 17, 26, 31;
    --bg-2: 29, 42, 49;
    --hover: #2c3d45;
    --text: #d3e0e2;
    --strong: #f1f8f8;
    --muted: #7e969c;
    --accent: #6fd3c0;
    --green: #6fd3c0;
    --yellow: #e3b657;
    --red: #e8705a;
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
    border-radius: 10px;
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
    font-size: 17px;
    font-weight: 600;
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
    border-radius: 10px;
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
    font-size: 13px;
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
  h1 {
    margin: 2px 0 2px;
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
    margin-top: 6px;
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
  .reward b {
    font-size: 14px;
    color: var(--strong);
  }
  .reward span {
    font-size: 12px;
    color: var(--muted);
  }
  .reward {
    display: grid;
    gap: 1px;
    margin-top: 10px;
    line-height: 1.3;
  }
  /* the route as a trail: the current step is the title, the steps ahead follow behind arrows */
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
    flex: none;
    width: 14px;
    font-size: 12px;
    text-align: center;
    color: var(--muted);
  }
  .notable .glyph {
    color: var(--yellow);
  }
  .keystone .glyph {
    color: var(--red);
  }
  .mastery .glyph,
  .jewel .glyph {
    color: var(--accent);
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
  .settings {
    position: absolute;
    right: 18px;
    bottom: 19vh; /* above the game's mana orb */
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 300px;
    max-height: calc(81vh - 18px);
    overflow-y: auto;
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
    stroke: #25353c;
    stroke-width: 7;
  }
  .mini .l.now,
  .mini .l.later {
    stroke: #5d7a84;
  }
  .mini .n {
    fill: #3a4f58;
  }
  .mini .n.later {
    fill: none;
    stroke: #5d7a84;
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
    border-radius: 6px;
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
  .where {
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .grp {
    margin-top: 8px;
    font-size: 13px;
    line-height: 1.35;
    color: var(--strong);
  }
  .grp .support {
    color: var(--muted);
  }
  .grp .new {
    color: var(--accent);
    font-weight: 600;
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
    border-radius: 10px;
    background: var(--accent);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    font-size: 13px;
    font-weight: 600;
    color: #0b1a1d;
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
  .hint,
  .foot {
    margin-top: 8px;
    font-size: 11px;
    color: var(--muted);
  }
  .foot {
    margin-top: 0;
    text-align: right;
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
  .slider select {
    padding: 4px 6px;
    border: 0;
    border-radius: 6px;
    background: rgb(var(--bg-0));
    color: var(--text);
    font: inherit;
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
    border-radius: 6px;
    background: #3a4f58;
    color: #fff;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.1s;
  }
  button:hover:not(:disabled) {
    background: #5d7a84;
  }
  .row button:first-child {
    background: var(--accent);
  }
  .row button:first-child:hover:not(:disabled) {
    background: #52b5a3;
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
  }
</style>
