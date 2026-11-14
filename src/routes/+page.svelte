<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Widget from "$lib/Widget.svelte";
  import Settings from "$lib/Settings.svelte";
  import { i18n, t, type Key } from "$lib/i18n.svelte";
  import { keyOf, loadBinds } from "$lib/hotkeys.svelte";
  import { BANDITS, layout, loadLayout, saveSoon, setBandit, sizeKey } from "$lib/layout.svelte";
  import type { Kind, Snapshot } from "$lib/types";

  const GLYPH: Record<Kind, string> = { Normal: "·", Notable: "◆", Keystone: "★", Mastery: "◇", Jewel: "●", Root: "·" };

  let edit = $state(false);
  let poeFound = $state(true);
  let snap = $state<Snapshot | null>(null);
  let panel = $state(false);

  const route = $derived(snap?.views.route ?? null);
  const plan = $derived(snap?.views.plan ?? null);
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
  // the stage's passives by where they stand for this character: taken, to take now, waiting for more points
  const rows = $derived.by(() => {
    const ps = plan?.passives ?? [];
    return { now: ps.filter((p) => p.available && !p.taken), later: ps.filter((p) => !p.available && !p.taken), taken: ps.filter((p) => p.taken) };
  });
  const nowShown = $derived(panel ? rows.now : rows.now.slice(1, COMPACT));
  const barPart = (n: number) => `flex: ${n} 1 0`;
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

  let ready = $state(false);
  $effect(() => void (i18n.lang = layout.settings.lang));
  // the first time the settings are opened the play-mode hint has done its job
  $effect(() => {
    if (edit && ready && !layout.settings.onboarded) {
      layout.settings.onboarded = true;
      saveSoon();
    }
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

  onMount(() => {
    loadLayout().then(() => (ready = true));
    void loadBinds();
    void invoke<boolean>("is_edit").then((on) => (edit = on));
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
        <p class="where">{t("act", { n: route.act })} · {route.act_step} / {route.act_total}</p>
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
        {#if snap?.state.town}
          {#if route.recipes.length}<p class="next">{t("recipes", { list: route.recipes.join(" · ") })}</p>{/if}
          {#if route.act === 2}
            <p class="next bandit">
              <span>{t("banditTitle")}</span>
              <!-- clicks pass through the overlay outside edit mode, so it is a choice only there -->
              {#if edit}
                <select value={layout.settings.bandit} onchange={(e) => setBandit(e.currentTarget.value as typeof layout.settings.bandit)} aria-label={t("banditTitle")}>
                  {#each BANDITS as b}<option value={b}>{t(`bandit${b[0].toUpperCase()}${b.slice(1)}` as Key)}</option>{/each}
                </select>
              {:else}
                <b>{t(`bandit${layout.settings.bandit[0].toUpperCase()}${layout.settings.bandit.slice(1)}` as Key)}</b>
              {/if}
            </p>
          {/if}
        {/if}
      </section>
      </Widget>
    {/if}

    {#if plan}
      <Widget id="passive" {edit}>
      <section class="card" class:panel>
        <header>
          <span class="eyebrow">{t("passivesTitle")}</span>
          <span class="meta">{t("stageOf", { n: plan.stage + 1, total: plan.stages.length })}</span>
        </header>
        <p class="stage">{#if !panel}{plan.stage_title}{/if}{#if plan.manual}<span class="pill">{t("manual")}</span>{/if}</p>
        {#if panel}
          <p class="stages">
            {#each plan.stages as s, i}<span class:cur={i === plan.stage} class:past={i < plan.stage}>{i < plan.stage ? "✓ " : ""}{s}</span>{/each}
          </p>
        {/if}
        {#if rows.now[0]}
          <p class="take"><span class="go">▶</span><span>{t("takeNext")} <b>{rows.now[0].name}</b></span></p>
        {:else if plan.passives.length}
          <p class="take idle">{rows.taken.length === plan.passives.length ? t("stageDone") : rows.later[0]?.unlock ? t("waitLevel", { n: rows.later[0].unlock }) : t("waitPoints")}</p>
        {/if}
        {#if plan.passives_total}
          <div class="bar" aria-hidden="true">
            <i class="d" style={barPart(rows.taken.length)}></i><i class="n" style={barPart(rows.now.length)}></i><i style={barPart(Math.max(0, plan.passives_total - rows.taken.length - rows.now.length))}></i>
          </div>
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
          <p class="legend"><span class="k done">●</span>{t("legendDone")}<span class="k now">●</span>{t("legendNow")}<span class="k later">○</span>{t("legendLater")}<span class="k ringk">◎</span>{t("legendNext")}</p>
        {/if}
        {#if plan.budget !== null && panel}
          <p class="next">{t("pointsToSpend", { n: Math.max(0, plan.budget - plan.spent_before) })}</p>
        {/if}
        {#if plan.sync}
          <p class="next">{t("syncLine", { name: plan.sync.name, lvl: plan.sync.level, n: plan.sync.missing_total, x: plan.sync.extra })}</p>
        {/if}
        {#if plan.passives.length === 0}
          <p class="next">{t("noPassives")}</p>
        {/if}
        {#snippet row(p: (typeof rows.now)[number], mark: string)}
          <li class={p.kind.toLowerCase()}>
            <span class="mark">{mark}</span>
            <span class="glyph">{GLYPH[p.kind]}</span>
            <span class="name">{p.name}</span>
            {#if p.unlock}<span class="unlock">{t("unlockLevel", { n: p.unlock })}</span>{/if}
            {#if panel && p.kind !== "Normal" && p.stats[0]}<span class="stat">{p.stats[0]}</span>{/if}
          </li>
        {/snippet}
        {#if nowShown.length}
          {#if panel}<h4 class="sec">{t("secNow")} · {rows.now.length}</h4>{/if}
          <ol class="list now">{#each nowShown as p}{@render row(p, "▶")}{/each}</ol>
        {/if}
        {#if panel && rows.later.length}
          <h4 class="sec">{t("secLater")} · {rows.later.length}</h4>
          <ol class="list later">{#each rows.later as p}{@render row(p, "○")}{/each}</ol>
        {/if}
        {#if panel && rows.taken.length}
          <h4 class="sec">{t("secTaken")} · {rows.taken.length}</h4>
          <p class="doneline"><span class="ok">✓</span> {rows.taken.map((p) => p.name).join(" · ")}</p>
        {/if}
        {#if !panel}
          {@const more = rows.now.length + rows.later.length - nowShown.length - (rows.now.length ? 1 : 0)}
          <p class="next sub">
            {#if rows.taken.length}<span class="ok">✓ {t("takenCount", { n: rows.taken.length })}</span>{/if}
            {#if more > 0}{rows.taken.length ? " · " : ""}{t("moreHint", { n: more, key: keyOf("panel") })}{/if}
          </p>
        {/if}
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
            {#if plan.sync}
              <span class="meta" class:gain={!plan.sync.missing_gems.length} class:warn={plan.sync.missing_gems.length > 0}>{plan.sync.missing_gems.length ? t("gemsMissing", { n: plan.sync.missing_gems.length }) : t("gemsAll")}</span>
            {:else if plan.new_gems.length}
              <span class="meta gain">{t("newGems", { n: plan.new_gems.length })}</span>
            {/if}
          </header>
          <p class="stage">{plan.stage_title}</p>
          {#snippet gem(g: { name: string; support: boolean })}
            {@const miss = plan?.sync?.missing_gems.includes(g.name) ?? false}
            <span class="g" class:new={plan?.new_gems.includes(g.name)} class:miss>{#if plan?.sync}<i>{miss ? "✗" : "✓"}</i>{:else if plan?.new_gems.includes(g.name)}<i>＋</i>{/if}{g.name}</span>
          {/snippet}
          {#each links.slice(0, panel ? 12 : 4) as grp}
            <div class="link">
              <p class="main">{@render gem(grp[0])}</p>
              {#if grp.length > 1}<p class="sup">{#each grp.slice(1) as g, i}{@render gem(g)}{i < grp.length - 2 ? " · " : ""}{/each}</p>{/if}
            </div>
          {/each}
          {#if links.length > (panel ? 12 : 4)}<p class="next">{t("moreLinks", { n: links.length - (panel ? 12 : 4) })}</p>{/if}
        </section>
        </Widget>
      {/if}

      {#if changedGear.length && shop}
        <Widget id="gear" {edit}>
        <section class="card">
          <header>
            <span class="eyebrow">{t("gear")}</span>
            <span class="meta">{t("gearCount", { n: changedGear.length })}</span>
          </header>
          <p class="stage">{plan.stage_title}</p>
          <ul class="list gear">
            {#each changedGear.slice(0, panel ? changedGear.length : 6) as g}
              <li class:copied={copied === g.slot}>
                <button class="copy" title={t("copyTitle")} onclick={() => copySearch(g)}>
                  <span class="slot">{g.slot}</span><span class="name">{g.name}{#if g.base}<span class="base"> · {g.base}</span>{/if}</span>
                </button>
              </li>
            {/each}
          </ul>
          {#if !panel && changedGear.length > 6}<p class="next sub">{t("moreHint", { n: changedGear.length - 6, key: keyOf("panel") })}</p>{/if}
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

  <!-- bottom right: the top left is where people keep their own panels -->
  <aside class="settings">
    {#if edit}
      <Settings {snap} {poeFound} />
    {:else}
      {#if snap?.log === "not-found"}
        <p class="foot warn">{t("logHint", { key: keyOf("edit") })}</p>
      {:else}
        <p class="foot">{t("footer", { edit: keyOf("edit"), hide: keyOf("hide") })}</p>
      {/if}
      {#if !poeFound}<p class="foot warn">{t("noPoe")}</p>{/if}
    {/if}
  </aside>
</main>

<style>
  /* Game-first look. While playing there are no panels: text sits straight on the game with a soft edge for
     legibility and a very faint scrim (the Opacity setting). Only what you operate (settings, the T panel, toasts)
     is a solid surface. One neutral ink, one sage accent that means "now" or "gain", one amber and one red for
     warnings. One typeface; three sizes do the work: 17 for the step, 13-14 for text, 12 for notes. */
  main {
    position: fixed;
    inset: 0;
    padding: 18px;
    outline: 1px dashed transparent;
    outline-offset: -3px;
    transition: outline-color 0.15s ease-out;
  }
  main.edit {
    outline-color: rgba(236, 233, 226, 0.3);
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 300px;
    margin-top: 40px;
    text-shadow: var(--edge);
  }
  .compact .col {
    gap: 6px;
  }

  /* top centre: level and zone as plain text */
  .hud {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    text-shadow: var(--edge);
  }
  .lvl {
    font-size: 15px;
    font-weight: 600;
    color: var(--accent);
  }
  .area {
    color: var(--ink);
  }
  .alvl {
    color: var(--ink-3);
  }
  .status {
    color: var(--warn);
  }
  .sep {
    display: none;
  }

  /* the cards: no frame, a faint scrim */
  .card {
    padding: 8px 10px;
    border-radius: 6px;
    background: rgba(0, 0, 0, calc(var(--alpha, 0.72) * 0.4));
    animation: in 0.14s ease-out;
  }
  .compact .card {
    padding: 6px 8px;
  }
  p {
    margin: 0;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }
  .eyebrow {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .meta {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--ink-3);
    white-space: nowrap;
  }
  .meta.gain {
    color: var(--accent);
  }

  /* route */
  .where {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--ink-3);
  }
  h1 {
    margin: 2px 0 2px;
    font-size: 17px;
    font-weight: 600;
    line-height: 1.25;
    color: var(--ink);
  }
  .compact h1 {
    font-size: 15px;
  }
  .step {
    animation: tick 0.3s ease-out;
  }
  .label {
    font-size: 13px;
    color: var(--ink-2);
  }
  .points {
    margin-top: 4px;
    font-size: 13px;
    color: var(--accent);
  }
  .reward {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: 6px;
    line-height: 1.3;
  }
  .reward b {
    font-size: 14px;
    font-weight: 600;
    color: var(--ink);
  }
  .reward span {
    font-size: 12px;
    color: var(--ink-2);
  }
  .next {
    margin-top: 4px;
    font-size: 13px;
    color: var(--ink-2);
  }
  .bandit {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  /* lists: passives, gear */
  .list {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
    font-size: 13px;
  }
  .list li {
    display: flex;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .glyph {
    flex: none;
    width: 12px;
    font-size: 11px;
    text-align: center;
    color: var(--ink-3);
  }
  .notable .glyph {
    color: var(--warn);
  }
  .keystone .glyph {
    color: var(--bad);
  }
  .mastery .glyph,
  .jewel .glyph {
    color: var(--accent);
  }
  .name {
    color: var(--ink);
  }
  .stat {
    flex-basis: 100%;
    padding-left: 20px;
    font-size: 12px;
    color: var(--ink-2);
  }
  /* passives: where you are in the stage */
  .stage {
    margin-top: 2px;
    font-size: 12px;
    color: var(--ink-3);
  }
  .pill {
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--ink-2);
  }
  .take {
    display: flex;
    gap: 6px;
    align-items: baseline;
    margin-top: 8px;
    font-size: 14px;
    color: var(--ink-2);
  }
  .take b {
    font-weight: 600;
    color: var(--ink);
  }
  .take .go {
    color: var(--accent);
  }
  .take.idle {
    font-size: 13px;
  }
  .bar {
    display: flex;
    gap: 2px;
    height: 4px;
    margin-top: 6px;
  }
  .bar i {
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.16);
  }
  .bar i.d {
    background: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .bar i.n {
    background: var(--accent);
  }
  .unlock {
    margin-left: auto;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--ink-3);
  }
  .sec {
    margin: 10px 0 0;
    font-size: 11px;
    font-weight: 600;
    color: var(--ink-3);
  }
  .panel .sec {
    grid-column: 1 / -1;
  }
  .mark {
    flex: none;
    width: 12px;
    font-size: 10px;
    text-align: center;
    color: var(--ink-3);
  }
  .list.now .mark {
    color: var(--accent);
  }
  .list.now .name {
    font-weight: 600;
  }
  .list.later .name,
  .list.later .glyph {
    color: var(--ink-3);
  }
  .ok {
    color: var(--accent);
  }
  .doneline {
    margin-top: 4px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--ink-2);
  }
  .sub {
    font-size: 12px;
    color: var(--ink-3);
  }
  .stages .past {
    color: var(--ink-2);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 6px;
    margin-top: 6px;
    font-size: 11px;
    color: var(--ink-3);
  }
  .legend .k {
    margin-left: 6px;
  }
  .legend .k:first-child {
    margin-left: 0;
  }
  .legend .k.done {
    color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .legend .k.now,
  .legend .k.ringk {
    color: var(--accent);
  }

  /* gems: one block per link, the skill first */
  .link {
    margin-top: 6px;
  }
  .link p {
    font-size: 13px;
    line-height: 1.35;
  }
  .link .main {
    font-weight: 600;
    color: var(--ink);
  }
  .link .sup {
    padding-left: 12px;
    color: var(--ink-2);
  }
  .g i {
    margin-right: 4px;
    font-size: 11px;
    font-style: normal;
    color: var(--ink-3);
  }
  .g.new,
  .g.new i {
    color: var(--accent);
  }
  .g.miss,
  .g.miss i {
    color: var(--warn);
  }
  p.mastery {
    margin-top: 6px;
    font-size: 12px;
    color: var(--accent);
  }
  .slot {
    min-width: 84px;
    font-size: 12px;
    color: var(--ink-3);
  }
  .gear .slot {
    min-width: 84px;
  }
  .gear .name {
    font-weight: 600;
  }
  .gear .base {
    color: var(--ink-2);
  }
  button.copy {
    display: flex;
    gap: 8px;
    align-items: baseline;
    width: 100%;
    padding: 0;
    border: 0;
    font: inherit;
    background: none;
    color: inherit;
    text-align: left;
    text-shadow: inherit;
  }
  button.copy:hover:not(:disabled) {
    background: none;
    color: var(--accent);
  }
  .copied .name {
    color: var(--accent);
  }
  .hint,
  .foot {
    margin-top: 6px;
    font-size: 11px;
    color: var(--ink-3);
  }
  .foot {
    margin-top: 0;
    text-align: right;
    text-shadow: var(--edge);
  }
  .warn {
    color: var(--warn);
  }

  /* toasts */
  .toasts {
    position: absolute;
    top: 38px;
    left: 50%;
    transform: translateX(-50%);
    display: grid;
    gap: 6px;
    justify-items: center;
  }
  .toast {
    padding: 6px 12px;
    border-radius: 6px;
    background: rgba(var(--surface), 0.92);
    font-size: 13px;
    color: var(--ink);
    animation: in 0.14s ease-out;
  }

  /* on demand: the T panel, notes, and everything you operate. These are solid. */
  .card.panel {
    width: 560px;
  }
  .card.panel,
  .card.notes {
    background: rgba(var(--surface), 0.94);
    text-shadow: none;
  }
  .panel .list {
    grid-template-columns: 1fr 1fr;
    column-gap: 12px;
  }
  .stages {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 8px;
    font-size: 12px;
  }
  .stages span {
    color: var(--ink-3);
  }
  .stages .cur {
    color: var(--ink);
    font-weight: 600;
  }
  .mini {
    display: block;
    width: 100%;
    margin-top: 8px;
    border-radius: 4px;
    background: rgb(12, 13, 15);
  }
  .mini .l {
    stroke: color-mix(in srgb, var(--accent) 35%, #1b1e22);
    stroke-width: 7;
  }
  .mini .l.now,
  .mini .l.later {
    stroke: #59616a;
  }
  .mini .n {
    fill: color-mix(in srgb, var(--accent) 45%, #1b1e22);
  }
  .mini .n.later {
    fill: none;
    stroke: #59616a;
    stroke-width: 6;
  }
  .mini .n.now {
    fill: var(--ink);
  }
  .mini .n.now.notable {
    fill: var(--warn);
  }
  .mini .n.now.keystone {
    fill: var(--bad);
  }
  .mini .n.now.mastery,
  .mini .n.now.jewel {
    fill: var(--accent);
  }
  .mini .ring {
    fill: none;
    stroke: var(--ink);
    stroke-width: 7;
  }
  .notes-body {
    max-height: 260px;
    margin-top: 8px;
    overflow: hidden;
    font-size: 12px;
    line-height: 1.4;
    white-space: pre-wrap;
    color: var(--ink);
  }

  /* settings: bottom right, above the game's mana orb */
  .settings {
    position: absolute;
    right: 18px;
    bottom: 19vh;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    width: 380px;
  }

  @keyframes in {
    from {
      opacity: 0;
    }
  }
  @keyframes tick {
    from {
      opacity: 0;
      transform: translateY(3px);
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
