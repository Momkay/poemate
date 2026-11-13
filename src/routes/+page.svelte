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
        passives: { name: string; kind: Kind; stats: string[]; known: boolean; available: boolean }[];
        masteries: [string, string[]][];
        new_gems: string[];
        gems: { name: string; support: boolean }[][];
        gear: { slot: string; name: string; base: string; changed: boolean }[];
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
  const changedGear = $derived(plan?.gear.filter((g) => g.changed) ?? []);

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

  <div class="col">
    {#if route}
      <Widget id="route" {edit}>
      <section class="card">
        <header><span class="eyebrow">Act {route.act}</span><span class="badge">{route.index + 1} / {route.total}</span></header>
        <h1>{route.step}</h1>
        {#if route.label}<p class="label">{route.label}</p>{/if}
        {#each route.hints as h}<p class="hint-line">{h}</p>{/each}
        {#if route.passive_points > 0}
          <p class="points">+{route.passive_points} passive point{route.passive_points > 1 ? "s" : ""}</p>
        {/if}
        {#each route.rewards as r}
          <p class="reward"><b>{r.how === "Take" ? "Take" : "Buy"}</b> {r.gem} <span>· {r.npc}</span></p>
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

      {#if plan.new_gems.length}
        <Widget id="gems" {edit}>
        <section class="card">
          <header><span class="eyebrow">New gems</span></header>
          <div class="chips">{#each plan.new_gems as g}<span class="chip">{g}</span>{/each}</div>
        </section>
        </Widget>
      {/if}

      {#if changedGear.length}
        <Widget id="gear" {edit}>
        <section class="card">
          <header><span class="eyebrow">Gear to look for</span></header>
          <ul class="list">
            {#each changedGear.slice(0, 6) as g}
              <li><span class="slot">{g.slot}</span><span class="name">{g.name}{g.base ? ` · ${g.base}` : ""}</span></li>
            {/each}
          </ul>
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
  :global(html, body) {
    margin: 0;
    background: transparent;
    overflow: hidden;
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
    color: #e9eaf0;
    user-select: none;
  }
  main {
    position: fixed;
    inset: 0;
    padding: 18px;
    outline: 2px dashed transparent;
    outline-offset: -2px;
    transition: outline-color 0.15s ease-out;
  }
  main.edit {
    outline-color: rgba(120, 170, 255, 0.6);
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 300px;
    margin-top: 56px;
  }
  .hud,
  .card {
    background: rgba(14, 15, 20, var(--alpha, 0.72));
    backdrop-filter: blur(14px) saturate(1.2);
    border: 1px solid rgba(255, 255, 255, 0.08);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }
  .hud {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 14px;
    border-radius: 999px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .lvl {
    font-weight: 600;
  }
  .alvl {
    opacity: 0.6;
  }
  .status {
    color: #ffc27a;
  }
  .sep {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.35);
  }
  .compact .card {
    padding: 8px 10px;
  }
  .compact .col {
    gap: 6px;
  }
  .compact h1 {
    font-size: 15px;
  }
  .slider {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 8px;
    font-size: 12px;
  }
  .slider input {
    width: 140px;
    margin: 0;
    padding: 0;
  }
  .card {
    padding: 12px 14px;
    border-radius: 12px;
    animation: in 0.18s ease-out;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .eyebrow {
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    opacity: 0.6;
  }
  .badge {
    font-size: 11px;
    padding: 1px 8px;
    border-radius: 999px;
    background: rgba(255, 170, 60, 0.18);
    color: #ffc27a;
    white-space: nowrap;
  }
  h1 {
    margin: 8px 0 2px;
    font-size: 17px;
    font-weight: 600;
    text-shadow: 0 0 14px rgba(255, 170, 60, 0.35);
  }
  p {
    margin: 0;
  }
  .label {
    font-size: 12px;
    opacity: 0.75;
  }
  .hint-line {
    margin-top: 3px;
    font-size: 12px;
    color: #c9d4ff;
    opacity: 0.85;
  }
  .points {
    margin-top: 5px;
    font-size: 12.5px;
    color: #ffd27a;
  }
  .locked {
    opacity: 0.4;
  }
  .reward {
    margin-top: 5px;
    font-size: 12.5px;
    color: #b7dcff;
  }
  .reward b {
    color: #7fd6a2;
  }
  .reward span {
    opacity: 0.6;
  }
  .next {
    margin-top: 4px;
    font-size: 12px;
    opacity: 0.5;
  }
  .list {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
    font-size: 12.5px;
  }
  .list li {
    display: flex;
    gap: 6px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .glyph {
    width: 12px;
    text-align: center;
    opacity: 0.7;
  }
  .notable .glyph {
    color: #ffc27a;
    opacity: 1;
  }
  .keystone .glyph {
    color: #ff8f8f;
    opacity: 1;
  }
  .card.panel {
    width: 560px;
  }
  .panel .list {
    grid-template-columns: 1fr 1fr;
    column-gap: 14px;
  }
  .counts,
  .stages {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 6px;
    font-size: 12px;
  }
  .counts .notable {
    color: #ffc27a;
  }
  .counts .keystone {
    color: #ff8f8f;
  }
  .stages {
    gap: 6px;
    font-size: 10.5px;
    opacity: 0.55;
  }
  .stages .cur {
    color: #ffc27a;
    opacity: 1;
  }
  .mastery {
    margin-top: 6px;
    font-size: 11.5px;
    color: #b9a7ff;
  }
  .stat {
    flex-basis: 100%;
    padding-left: 18px;
    font-size: 11px;
    opacity: 0.55;
  }
  .slot {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    opacity: 0.55;
    min-width: 72px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 8px;
  }
  .chip {
    font-size: 11.5px;
    padding: 2px 9px;
    border-radius: 999px;
    background: rgba(120, 190, 255, 0.14);
    color: #b7dcff;
  }
  .hint,
  .foot {
    margin-top: 8px;
    font-size: 10px;
    opacity: 0.45;
  }
  .foot {
    padding-left: 4px;
  }
  .warn {
    color: #ffc27a;
    opacity: 0.85;
  }
  .editbox input {
    width: 100%;
    box-sizing: border-box;
    margin-top: 8px;
    padding: 6px 8px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(0, 0, 0, 0.35);
    color: inherit;
    font: inherit;
    font-size: 12px;
    user-select: text;
  }
  .row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  button {
    padding: 4px 10px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.08);
    color: inherit;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.16);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      animation: none;
    }
  }
</style>
