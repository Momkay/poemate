<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { t, type Key } from "./i18n.svelte";
  import { BANDITS, layout, resetLayout, saveSoon, setBandit } from "./layout.svelte";
  import { hk, keyOf, pretty, comboOf, setBind, resetBinds } from "./hotkeys.svelte";
  import type { Snapshot } from "./types";

  let { snap, poeFound, tab = $bindable("setup") }: { snap: Snapshot | null; poeFound: boolean; tab?: "setup" | "route" | "display" | "keys" | "about" } = $props();

  const TABS = [
    ["setup", "tabSetup"],
    ["route", "tabRoute"],
    ["display", "tabDisplay"],
    ["keys", "tabKeys"],
    ["about", "tabAbout"],
  ] as const;
  let version = $state("");
  onMount(() => void getVersion().then((v) => (version = v)).catch(() => {}));

  const plan = $derived(snap?.views.plan ?? null);
  const route = $derived(snap?.views.route ?? null);
  const logOk = $derived(snap?.log === "ok");
  const logMissing = $derived(snap?.log === "not-found");
  const close = () => invoke("exit_edit");

  // ---- setup: game log and build ----
  let logPath = $state("");
  let logMsg = $state("");
  async function browseLog() {
    const f = await open({ filters: [{ name: "Client.txt", extensions: ["txt"] }] }).catch(() => null);
    if (typeof f === "string") {
      logPath = f;
      await pickLog();
    }
  }
  async function pickLog() {
    try {
      await invoke("set_log_path", { path: logPath });
      logMsg = t("logPickSaved");
      logPath = "";
    } catch (e) {
      logMsg = String(e);
    }
  }

  let buildInput = $state("");
  let buildMsg = $state("");
  let busy = $state(false);
  async function importBuild() {
    if (!buildInput.trim() || busy) return;
    busy = true;
    buildMsg = t("importing");
    try {
      await invoke("import_build", { input: buildInput });
      buildMsg = "";
      buildInput = "";
    } catch (e) {
      buildMsg = String(e);
    } finally {
      busy = false;
    }
  }
  const removeBuild = () => invoke("remove_build").then(() => (buildMsg = ""));

  let syncMsg = $state("");
  let syncing = $state(false);
  let synced = $state("");
  async function removeCharacter() {
    await invoke("remove_character");
    synced = syncMsg = "";
    layout.settings.account = layout.settings.character = "";
    saveSoon();
  }
  async function syncCharacter() {
    if (syncing) return;
    syncing = true;
    syncMsg = t("importing");
    try {
      const what = await invoke<string>("sync_character", { account: layout.settings.account, character: layout.settings.character });
      syncMsg = "";
      synced = what;
    } catch (e) {
      syncMsg = String(e);
    } finally {
      syncing = false;
    }
  }

  let cookie = $state("");
  let cookieSaved = $state(false);
  let cookieMsg = $state("");
  onMount(() => void invoke<boolean>("session_cookie_has").then((v) => (cookieSaved = v)).catch(() => {}));
  async function setCookie(value: string) {
    try {
      await invoke("session_cookie_set", { value });
      cookieSaved = !!value.trim();
      cookie = "";
      cookieMsg = "";
    } catch (e) {
      cookieMsg = String(e);
    }
  }

  // ---- route ----
  const setQuests = (q: string) => {
    layout.settings.quests = q as "essential" | "all";
    saveSoon();
    void invoke("quest_mode", { allQuests: q === "all" });
  };
  // the Rust side clamps to ±10 as well
  const setAdjust = (d: number) => {
    layout.settings.points_adjust = Math.max(-10, Math.min(10, layout.settings.points_adjust + d));
    saveSoon();
    void invoke("points_adjust", { n: layout.settings.points_adjust });
  };

  // ---- display ----
  const setDensity = (d: string) => {
    layout.settings.density = d as "cozy" | "compact";
    saveSoon();
  };
  const setLang = (l: string) => {
    layout.settings.lang = l as "en" | "nl";
    saveSoon();
  };

  // ---- keys ----
  let capturing = $state("");
  let bindMsg = $state("");
  async function capture(e: KeyboardEvent, id: string) {
    e.preventDefault();
    if (e.key === "Escape") {
      e.stopPropagation(); // cancels the capture, not the panel
      capturing = "";
      return;
    }
    const combo = comboOf(e);
    if (!combo) return;
    bindMsg = await setBind(id, combo);
    capturing = "";
  }
  const resetKeys = () => resetBinds().then(() => (bindMsg = ""));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && !capturing && close()} />

{#snippet seg(options: [string, string][], value: string, pick: (v: string) => void)}
  <div class="seg" role="group">
    {#each options as [v, label]}
      <button class:on={value === v} aria-pressed={value === v} onclick={() => pick(v)}>{label}</button>
    {/each}
  </div>
{/snippet}

<section class="panel" aria-label={t("settingsTitle")}>
  <header class="head">
    <svg class="mark" viewBox="0 0 512 512" aria-hidden="true">
      <path d="M166 388V106H252a94 94 0 0 1 0 188H166" fill="none" stroke="currentColor" stroke-width="52" stroke-linecap="round" stroke-linejoin="round" />
      <circle class="node" cx="166" cy="388" r="44" />
    </svg>
    <div class="title"><b>PoeMate</b><span>{t("settingsTitle")}</span></div>
  </header>

  <div class="tabs" role="tablist">
    {#each TABS as [k, label]}
      <button role="tab" aria-selected={tab === k} class:on={tab === k} onclick={() => (tab = k)}>{t(label)}</button>
    {/each}
  </div>

  <div class="body" role="tabpanel">
    {#if tab === "setup"}
      <div class="group">
        <h3>{t("gameTitle")}</h3>
        <div class="row">
          <span class="dot" class:ok={logOk} class:warn={!logOk && !!snap && snap.log !== "searching"}></span>
          <div class="l">
            <span>{logOk ? t("logFound") : logMissing ? t("logMissing") : snap?.log === "error" ? t("logError") : t("logSearching")}</span>
            {#if snap?.log_path}<small class="path" title={snap.log_path}>{snap.log_path}</small>{/if}
          </div>
        </div>
        <div class="row">
          <span class="dot" class:ok={poeFound} class:warn={!poeFound}></span>
          <div class="l"><span>{poeFound ? t("windowFound") : t("windowMissing")}</span></div>
        </div>
        {#if logMissing}
          <div class="row stack">
            <input bind:value={logPath} placeholder={t("logPickPlaceholder")} onkeydown={(e) => e.key === "Enter" && pickLog()} aria-label={t("logPickTitle")} />
            <button onclick={browseLog}>{t("logBrowse")}</button>
            <button class="primary" onclick={pickLog} disabled={!logPath.trim()}>{t("logPickBtn")}</button>
          </div>
          {#if logMsg}<p class="msg">{logMsg}</p>{/if}
          <p class="help">{t("logPickHint")}</p>
        {/if}
      </div>

      <div class="group">
        <h3>{t("build")}</h3>
        {#if plan}
          <div class="row">
            <span class="dot ok"></span>
            <div class="l"><span>{plan.ascendancy || plan.class}</span><small>{t("buildStages", { n: plan.stages.length })}</small></div>
            <button class="ghost" onclick={removeBuild}>{t("removeBuild")}</button>
          </div>
        {:else}
          <div class="row stack">
            <input bind:value={buildInput} placeholder={t("importPlaceholder")} onkeydown={(e) => e.key === "Enter" && importBuild()} aria-label={t("build")} />
            <button class="primary" onclick={importBuild} disabled={busy || !buildInput.trim()}>{t("importBtn")}</button>
          </div>
          {#if buildMsg}<p class="msg">{buildMsg}</p>{/if}
          <p class="help">{t("buildNoneHint")}</p>
        {/if}
      </div>

      <div class="group">
        <h3>{t("charTitle")}</h3>
        {#if synced}
          <div class="row">
            <span class="dot ok"></span>
            <div class="l"><span>{synced}</span></div>
            <button class="ghost" onclick={removeCharacter}>{t("charRemove")}</button>
          </div>
        {/if}
        <div class="row stack">
          <input bind:value={layout.settings.account} oninput={saveSoon} placeholder={t("charAccount")} aria-label={t("charAccount")} />
          <input bind:value={layout.settings.character} oninput={saveSoon} placeholder={t("charName")} aria-label={t("charName")} onkeydown={(e) => e.key === "Enter" && syncCharacter()} />
          <button class="primary" onclick={syncCharacter} disabled={syncing || !layout.settings.account.trim() || !layout.settings.character.trim()}>{t("charSync")}</button>
        </div>
        {#if syncMsg}<p class="msg">{syncMsg}</p>{/if}
        <p class="help">{t("charHint")}</p>
        <div class="row stack">
          {#if cookieSaved}
            <span class="l"><small>{t("cookieStored")}</small></span>
            <button class="ghost" onclick={() => setCookie("")}>{t("cookieRemove")}</button>
          {:else}
            <input type="password" autocomplete="off" bind:value={cookie} placeholder={t("cookieLabel")} aria-label={t("cookieLabel")} onkeydown={(e) => e.key === "Enter" && setCookie(cookie)} />
            <button class="primary" onclick={() => setCookie(cookie)} disabled={!cookie.trim()}>{t("cookieSave")}</button>
          {/if}
        </div>
        {#if cookieMsg}<p class="msg">{cookieMsg}</p>{/if}
        <p class="help">{t("cookieHint")}</p>
      </div>
    {:else if tab === "route"}
      <div class="group">
        <h3>{t("routeTitle")}</h3>
        <div class="row">
          <div class="l"><span>{t("questsTitle")}</span><small>{layout.settings.quests === "all" ? t("questsAllHint") : t("questsEssentialHint")}</small></div>
          {@render seg([["essential", t("questsEssential")], ["all", t("questsAll")]], layout.settings.quests, setQuests)}
        </div>
        <div class="row">
          <div class="l"><span>{t("banditTitle")}</span><small>{t("banditHint")}</small></div>
          <select value={layout.settings.bandit} onchange={(e) => setBandit(e.currentTarget.value as typeof layout.settings.bandit)} aria-label={t("banditTitle")}>
            {#each BANDITS as b}<option value={b}>{t(`bandit${b[0].toUpperCase()}${b.slice(1)}` as Key)}</option>{/each}
          </select>
        </div>
        <div class="row">
          <div class="l"><span>{t("adjustTitle")}</span><small>{t("adjustHint")}</small></div>
          <div class="pair">
            <button class="ghost" aria-label="−" onclick={() => setAdjust(-1)}>−</button>
            <span>{layout.settings.points_adjust > 0 ? "+" : ""}{layout.settings.points_adjust}</span>
            <button class="ghost" aria-label="+" onclick={() => setAdjust(1)}>+</button>
          </div>
        </div>
      </div>

      <div class="group">
        <h3>{t("where")}</h3>
        <div class="row">
          <div class="l"><span>{t("stepLabel")}</span><small>{route ? `${t("act", { n: route.act })} · ${route.act_step} / ${route.act_total}` : "–"}</small></div>
          <div class="pair">
            <button class="ghost" aria-label={t("back")} onclick={() => invoke("route_step", { delta: -1 })}>‹</button>
            <button class="ghost" aria-label={t("forward")} onclick={() => invoke("route_step", { delta: 1 })}>›</button>
          </div>
        </div>
        {#if plan}
          <div class="row">
            <div class="l"><span>{t("stageLabel")}</span><small>{plan.stage_title}{plan.manual ? ` · ${t("manual")}` : ""}</small></div>
            <div class="pair">
              <button class="ghost" aria-label={t("back")} onclick={() => invoke("plan_stage", { delta: -1 })}>‹</button>
              <button class="ghost" aria-label={t("forward")} onclick={() => invoke("plan_stage", { delta: 1 })}>›</button>
            </div>
          </div>
        {/if}
        <p class="help">{t("whereHint", { back: keyOf("step_prev"), fwd: keyOf("step_next") })}</p>
      </div>
    {:else if tab === "display"}
      <div class="group">
        <h3>{t("appearance")}</h3>
        <div class="row">
          <div class="l"><span>{t("backgroundLabel")}</span><small>{Math.round(layout.settings.opacity * 100)}%</small></div>
          <input class="range" type="range" min="0.2" max="1" step="0.05" bind:value={layout.settings.opacity} oninput={saveSoon} aria-label={t("backgroundLabel")} />
        </div>
        <div class="row">
          <div class="l"><span>{t("densityLabel")}</span></div>
          {@render seg([["cozy", t("cozy")], ["compact", t("compact")]], layout.settings.density, setDensity)}
        </div>
        <div class="row">
          <div class="l"><span>{t("language")}</span></div>
          {@render seg([["en", "English"], ["nl", "Nederlands"]], layout.settings.lang, setLang)}
        </div>
      </div>
      <div class="group">
        <h3>{t("layoutLabel")}</h3>
        <div class="row">
          <div class="l"><span>{t("cardsLabel")}</span><small>{t("layoutHint")}</small></div>
          <button class="ghost" onclick={resetLayout}>{t("resetLayout")}</button>
        </div>
      </div>
    {:else if tab === "keys"}
      <div class="group">
        <h3>{t("hotkeys")}</h3>
        {#each hk.binds as b}
          <div class="row tight">
            <div class="l"><span>{t(`act.${b.id}` as Key)}</span></div>
            <button
              class="key"
              class:rec={capturing === b.id}
              onclick={() => (capturing = b.id)}
              onkeydown={(e) => capturing === b.id && capture(e, b.id)}
              onblur={() => (capturing = "")}
            >{capturing === b.id ? t("pressKeys") : pretty(b.keys)}</button>
          </div>
        {/each}
        {#if bindMsg}<p class="msg">{bindMsg}</p>{/if}
        <p class="help">{t("hotkeyHint")}</p>
        <div class="end"><button class="ghost" onclick={resetKeys}>{t("resetKeys")}</button></div>
      </div>
    {:else}
      <div class="about">
        <svg class="mark big" viewBox="0 0 512 512" aria-hidden="true">
          <path d="M166 388V106H252a94 94 0 0 1 0 188H166" fill="none" stroke="currentColor" stroke-width="52" stroke-linecap="round" stroke-linejoin="round" />
          <circle class="node" cx="166" cy="388" r="44" />
        </svg>
        <b>PoeMate</b>
        <span>{t("aboutVersion", { v: version || "–" })}</span>
      </div>
      <p class="help">{t("aboutData")}</p>
      <p class="help"><b>{t("creditsTitle")}</b></p>
      <ul class="help">
        <li>exile-leveling (github.com/HeartofPhos/exile-leveling): {t("creditExile")}</li>
        <li>skilltree-export (github.com/grindinggear/skilltree-export): {t("creditTree")}</li>
        <li>Path of Building (github.com/PathOfBuildingCommunity/PathOfBuilding): {t("creditPob")}</li>
      </ul>
      <p class="help">{t("disclaimer")}</p>
    {/if}
  </div>

  <footer class="bar">
    <span class="esc"><kbd>Esc</kbd> {t("escHint")}</span>
    <button class="primary" onclick={close}>{t("close")}</button>
  </footer>
</section>

<style>
  .panel {
    --s3: 50, 55, 61;
    width: 380px;
    height: min(600px, calc(81vh - 18px));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 14px;
    background: rgb(var(--surface));
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.55);
    color: var(--ink);
    font-size: 13px;
    text-shadow: none;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 16px 12px;
  }
  .mark {
    flex: none;
    width: 22px;
    height: 22px;
    color: var(--ink);
  }
  .mark .node {
    fill: var(--accent);
  }
  .title b {
    display: block;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .title span {
    font-size: 12px;
    color: var(--ink-3);
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 12px 8px;
  }
  .tabs button {
    padding: 7px 10px;
    border-radius: 8px;
    background: none;
    color: var(--ink-2);
    font-weight: 500;
  }
  .tabs button:hover {
    color: var(--ink);
  }
  .tabs button.on {
    background: rgb(var(--surface-2));
    color: var(--ink);
  }

  .body {
    flex: 1;
    display: grid;
    align-content: start;
    gap: 22px;
    padding: 8px 16px 16px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #3a4048 transparent;
  }
  .group {
    display: grid;
    gap: 4px;
  }
  h3 {
    margin: 0 0 4px 2px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-3);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 12px;
    border-radius: 10px;
    background: rgb(var(--surface-2));
  }
  .row.stack {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    padding: 12px;
  }
  .l {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  .l small {
    font-size: 12px;
    line-height: 1.4;
    color: var(--ink-3);
  }
  .l small.path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.tight {
    min-height: 38px;
    padding: 4px 12px;
  }
  .help,
  .msg {
    margin: 4px 2px 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink-3);
  }
  .msg {
    color: var(--warn);
  }
  .end {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ink-3);
  }
  .dot.ok {
    background: var(--accent);
  }
  .dot.warn {
    background: var(--warn);
  }

  /* controls */
  button,
  input,
  select {
    font: inherit;
  }
  button {
    border: 0;
    cursor: pointer;
    transition: background 0.1s, color 0.1s;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button:focus-visible,
  input:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .primary {
    padding: 8px 16px;
    border-radius: 8px;
    background: var(--accent);
    color: #10231a;
    font-weight: 600;
  }
  .primary:hover:not(:disabled) {
    background: #a3d6b8;
  }
  .ghost {
    padding: 7px 12px;
    border-radius: 8px;
    background: rgb(var(--s3));
    color: var(--ink);
  }
  .ghost:hover:not(:disabled) {
    background: #3b4148;
  }
  .pair {
    display: flex;
    gap: 4px;
  }
  .pair .ghost {
    width: 34px;
    padding: 7px 0;
    font-size: 16px;
    line-height: 1;
  }
  input:not([type]),
  input[type="password"] {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border: 0;
    border-radius: 8px;
    background: rgb(var(--surface));
    color: var(--ink);
    user-select: text;
  }
  input::placeholder {
    color: var(--ink-3);
  }
  .range {
    width: 132px;
    margin: 0;
    accent-color: var(--accent);
  }
  select {
    padding: 7px 10px;
    border: 0;
    border-radius: 8px;
    background: rgb(var(--s3));
    color: var(--ink);
  }
  .seg {
    display: flex;
    padding: 2px;
    border-radius: 9px;
    background: rgb(var(--surface));
  }
  .seg button {
    padding: 5px 11px;
    border-radius: 7px;
    background: none;
    color: var(--ink-2);
  }
  .seg button.on {
    background: rgb(var(--s3));
    color: var(--ink);
  }
  .key {
    min-width: 96px;
    padding: 6px 10px;
    border-radius: 7px;
    background: rgb(var(--surface));
    color: var(--ink);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .key:hover {
    background: rgb(var(--s3));
  }
  .key.rec {
    background: var(--accent);
    color: #10231a;
  }

  .about {
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 20px 0 8px;
  }
  .about b {
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .about span {
    font-size: 12px;
    color: var(--ink-3);
  }
  .mark.big {
    width: 56px;
    height: 56px;
    margin-bottom: 8px;
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: rgb(var(--surface-2));
  }
  .esc {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--ink-3);
  }
  kbd {
    padding: 2px 6px;
    border-radius: 5px;
    background: rgb(var(--s3));
    color: var(--ink-2);
    font: inherit;
    font-size: 11px;
  }
</style>
