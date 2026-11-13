<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Widget from "$lib/Widget.svelte";
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
  const changedGear =$derived(plan?.gear.filter((g) => g.changed) ?? []);

  // level-up toasts: max 2 at once, 6 s each; the first level seen after start is not a level-up
  let toasts = $state<{ id: number; text: string }[]>([]);
  let lastLevel = 0;
  $effect(() => {
    const lv = snap?.state.level ?? 0;
    if (lastLevel && lv > lastLevel) {
      const id = Date.now();
      const pts = plan?.budget != null ? Math.max(0, plan.budget - plan.spent_before) : 0;
      toasts = [...toasts, { id, text: `Level ${lv}${pts ? ` · ${pts} passive points to spend` : ""}` }].slice(-2);
      setTimeout(() => (toasts = toasts.filter((t) => t.id !== id)), 6000);
    }
    if (lv) lastLevel = lv;
  });

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
    importMsg = "Importing…";
    try {
      const s = await invoke<{ class: string; ascendancy: string; stages: number }>("import_build", { input });
      importMsg = `Loaded ${s.ascendancy || s.class}, ${s.stages} stages`;
      input = "";
    } catch (e) {
      importMsg = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    loadLayout();
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
        {snap.log === "searching" ? "Looking for Client.txt…" : snap.log === "not-found" ? "Can't find Client.txt" : "Can't read Client.txt"}
      </span>
    {:else}
      <span class="lvl">{snap?.state.level || "–"}</span>
      <span class="sep"></span>
      <span class="area">{snap?.state.area_name || "Waiting for a zone…"}</span>
      {#if snap?.state.area_level}
        <span class="sep"></span>
        <span class="alvl">area {snap.state.area_level}</span>
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
        <header><span class="eyebrow">Act {route.act}</span><span class="badge">{route.index + 1} / {route.total}</span></header>
        <div class="bar"><i style:width="{((route.index + 1) / route.total) * 100}%"></i></div>
        <h1>{route.step}</h1>
        {#if route.label}<p class="label">{route.label}</p>{/if}
        {#each route.hints as h}<p class="hint-line">{h}</p>{/each}
        {#if route.passive_points > 0}
          <p class="points">+{route.passive_points} passive point{route.passive_points > 1 ? "s" : ""}</p>
        {/if}
        {#each route.rewards as r}
          <p class="reward">
            <span class="av {r.how === 'Take' ? 'take' : 'buy'}">{r.gem[0]}</span>
            <span class="rtxt"><b>{r.gem}</b><span>{r.how === "Take" ? "Take" : "Buy"} · {r.npc}</span></span>
          </p>
        {/each}
        {#each route.next as n}<p class="next">{n}</p>{/each}
      </section>
      </Widget>
    {/if}

    {#if plan}
      <Widget id="passive" {edit}>
      <section class="card" class:panel>
        <header>
          <span class="eyebrow">{plan.stage_title}{plan.manual ? " · manual" : ""}</span>
          <span class="badge">{plan.passives_available}/{plan.passives_total} now</span>
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
          <p class="next">≈ {Math.max(0, plan.budget - plan.spent_before)} points to spend (level − 1 + quests)</p>
        {/if}
        {#if plan.passives.length === 0}
          <p class="next">No new passives in this stage</p>
        {/if}
        <ol class="list">
          {#each panel ? plan.passives : plan.passives.slice(0, 8) as p}
            <li class={p.kind.toLowerCase()} class:locked={!p.available}>
              <span class="glyph">{GLYPH[p.kind]}</span>
              <span class="name">{p.name}</span>
              {#if p.kind !== "Normal" && p.stats[0]}<span class="stat">{p.stats[0]}</span>{/if}
            </li>
          {/each}
        </ol>
        {#if !panel && plan.passives_total > 8}<p class="next">+{plan.passives_total - 8} more · Ctrl+Shift+T for all</p>{/if}
        {#each plan.masteries as [name, stats]}
          <p class="mastery">◇ {name}: {stats.join(" · ")}</p>
        {/each}
        {#if plan.auto_order}<p class="hint">Order is auto-generated</p>{/if}
      </section>
      </Widget>

      {#if links.length}
        <Widget id="gems" {edit}>
        <section class="card">
          <header>
            <span class="eyebrow">Gems</span>
            {#if plan.new_gems.length}<span class="badge">{plan.new_gems.length} new</span>{/if}
          </header>
          {#each links.slice(0, panel ? 12 : 5) as grp}
            <div class="chips">
              {#each grp as g}
                <span class="chip" class:support={g.support} class:new={plan.new_gems.includes(g.name)}>{g.name}</span>
              {/each}
            </div>
          {/each}
          {#if links.length > (panel ? 12 : 5)}<p class="next">+{links.length - (panel ? 12 : 5)} more links</p>{/if}
        </section>
        </Widget>
      {/if}

      {#if changedGear.length}
        <Widget id="gear" {edit}>
        <section class="card">
          <header><span class="eyebrow">Gear to look for</span></header>
          <ul class="list">
            {#each changedGear.slice(0, 6) as g}
              <li class:copied={copied === g.slot}>
                <button class="copy" title="Copy the stash search" onclick={() => copySearch(g)}>
                  <span class="slot">{g.slot}</span><span class="name">{g.name}{g.base ? ` · ${g.base}` : ""}</span>
                </button>
              </li>
            {/each}
          </ul>
          <p class="hint">Click an item to copy its stash search</p>
        </section>
        </Widget>
      {/if}

      {#if plan.notes.trim() && panel}
        <Widget id="notes" {edit}>
        <section class="card notes">
          <header><span class="eyebrow">Build notes</span></header>
          <p class="notes-body">{plan.notes}</p>
        </section>
        </Widget>
      {/if}
    {:else if snap && !route}
      <section class="card"><p class="next">Load a build in edit mode (Ctrl+Shift+O)</p></section>
    {/if}

    {#if edit}
      <section class="card editbox">
        <header><span class="eyebrow">Import build</span></header>
        <input bind:value={input} placeholder="pobb.in / pastebin link or PoB code" onkeydown={(e) => e.key === "Enter" && doImport()} />
        <div class="row">
          <button onclick={doImport} disabled={busy}>Import</button>
          <button onclick={() => invoke("route_step", { delta: -1 })}>◀ step</button>
          <button onclick={() => invoke("route_step", { delta: 1 })}>step ▶</button>
        </div>
        {#if importMsg}<p class="next">{importMsg}</p>{/if}
        <label class="slider">Opacity
          <input type="range" min="0.2" max="1" step="0.05" bind:value={layout.settings.opacity} oninput={saveSoon} />
        </label>
        <div class="row">
          <button onclick={() => { layout.settings.density = layout.settings.density === "cozy" ? "compact" : "cozy"; saveSoon(); }}>
            {layout.settings.density === "cozy" ? "Compact" : "Cozy"}
          </button>
          <button onclick={resetLayout}>Reset layout</button>
        </div>
        <p class="hint">Drag cards to move · mouse wheel on a card to resize</p>
        <p class="hint">Ctrl+Shift+←/→ step · ↑/↓ stage · O edit · H hide</p>
      </section>
    {:else}
      <p class="foot">Ctrl+Shift+O edit · Ctrl+Shift+H hide</p>
    {/if}
    {#if !poeFound}
      <p class="foot warn">Path of Exile not found — start it in Windowed or Borderless mode</p>
    {/if}
  </div>
</main>

<style>
  /* Discord-overlay look: solid flat surfaces stepped by tone, one blurple accent, round avatars, no blur */
  :global(:root) {
    --bg-0: 17, 18, 20;
    --bg-1: 30, 31, 34;
    --bg-2: 43, 45, 49;
    --hover: #35373c;
    --text: #dbdee1;
    --strong: #f2f3f5;
    --muted: #949ba4;
    --blurple: #5865f2;
    --green: #23a55a;
    --yellow: #f0b232;
    --red: #f23f43;
  }
  :global(html, body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
    font-family: "gg sans", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
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
    border-radius: 999px;
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
    border-radius: 999px;
    background: var(--blurple);
    font-size: 12px;
    font-weight: 700;
    color: #fff;
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
    border-radius: 8px;
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
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .badge {
    min-width: 18px;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--blurple);
    font-size: 12px;
    font-weight: 700;
    line-height: 18px;
    text-align: center;
    color: #fff;
    white-space: nowrap;
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
    background: var(--blurple);
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
    border-radius: 6px;
    background: rgb(var(--bg-2));
  }
  .av {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
  }
  .av.take {
    background: var(--green);
  }
  .av.buy {
    background: var(--blurple);
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
    border-radius: 50%;
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
    background: var(--blurple);
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
    color: #a5aff9;
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
    fill: var(--blurple);
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
    border-radius: 999px;
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
    border-radius: 999px;
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
    border-radius: 999px;
    background: var(--blurple);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    font-size: 13px;
    font-weight: 600;
    color: #fff;
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
    outline: 2px solid var(--blurple);
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
    accent-color: var(--blurple);
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
    background: var(--blurple);
  }
  .row button:first-child:hover:not(:disabled) {
    background: #4752c4;
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
    .toast {
      animation: none;
    }
    .bar i {
      transition: none;
    }
  }
</style>
