<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  type PetState = {
    fullness: number;
    mood: string;
    xp: number;
    level: number;
    evolutionStage: string;
    evolutionBranch: string;
    sparks: number;
    ownedItems: string[];
    equippedCosmetic?: string | null;
    equippedFoodSkin?: string | null;
    furniture: { itemId: string; x: number }[];
    albumRecords: {
      key: string;
      stage: string;
      branch: string;
      reachedDay: string;
      level: number;
      xp: number;
      sparks: number;
      prestigeCount: number;
    }[];
    prestigeCount: number;
    xpBonusMultiplier: number;
    pendingFood: number;
    foodEarnedToday: number;
    bankedTokensToday: number;
    tokensPerFood: number;
    meterProgress: number;
  };
  type AppSettings = {
    onboardingComplete: boolean;
    starterEgg: string;
    claudeCodeEnabled: boolean;
    codexCliEnabled: boolean;
    openaiEnabled: boolean;
    petSize: number;
    monitorIndex: number;
    waylandFallback: boolean;
    trackingPaused: boolean;
    calmMode: boolean;
  };
  type TokenTotals = { input: number; output: number; cacheRead: number; total: number };
  type FoodStats = { today: number; week: number };
  type ShopItem = { id: string; label: string; kind: "Cosmetic" | "FoodSkin" | "Furniture" | "Heirloom"; priceSparks: number };
  type DashboardState = {
    pet: PetState;
    settings: AppSettings;
    providers: { claudeCodeDetected: boolean; codexCliDetected: boolean; openaiKeyConfigured: boolean };
    stats: { food: FoodStats; todayTokens: TokenTotals; weekTokens: TokenTotals; streakDays: number };
    monitorCount: number;
    shopCatalog: ShopItem[];
  };

  const starterEggs = [
    { id: "sprout", label: "Sprout", tone: "#a7f070" },
    { id: "ember", label: "Ember", tone: "#ef7d57" },
    { id: "bubble", label: "Bubble", tone: "#73eff7" },
  ];

  type UpdateStatus = "idle" | "checking" | "up-to-date" | "downloading" | "ready" | "error";

  let dashboard = $state<DashboardState | null>(null);
  let autostart = $state(false);
  let selectedEgg = $state("sprout");
  let busy = $state(false);
  let error = $state("");
  let updateStatus = $state<UpdateStatus>("idle");
  let updateVersion = $state("");
  let openaiApiKey = $state("");

  function formatNumber(value: number): string {
    return Math.round(value).toLocaleString();
  }

  async function refresh() {
    try {
      error = "";
      const [nextDashboard, nextAutostart] = await Promise.all([
        invoke<DashboardState>("get_dashboard_state"),
        invoke<boolean>("is_autostart_enabled"),
      ]);
      dashboard = nextDashboard;
      autostart = nextAutostart;
      selectedEgg = nextDashboard.settings.starterEgg;
    } catch (cause) {
      error = String(cause);
    }
  }

  async function saveSettings(next: AppSettings) {
    if (!dashboard) return;
    try {
      error = "";
      dashboard.settings = await invoke<AppSettings>("update_settings", { settings: next });
      await refresh();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function patchSettings(patch: Partial<AppSettings>) {
    if (!dashboard) return;
    await saveSettings({ ...dashboard.settings, ...patch });
  }

  async function finishOnboarding() {
    if (!dashboard) return;
    try {
      busy = true;
      error = "";
      dashboard.settings = await invoke<AppSettings>("complete_onboarding", { starterEgg: selectedEgg });
      await refresh();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = false;
    }
  }

  async function toggleAutostart() {
    try {
      error = "";
      autostart = await invoke<boolean>("set_autostart", { enabled: !autostart });
    } catch (cause) {
      error = String(cause);
    }
  }

  async function checkForUpdates() {
    try {
      updateStatus = "checking";
      error = "";
      const update = await check();
      if (!update) {
        updateStatus = "up-to-date";
        return;
      }
      updateVersion = update.version;
      updateStatus = "downloading";
      await update.downloadAndInstall();
      updateStatus = "ready";
    } catch (cause) {
      updateStatus = "error";
      error = String(cause);
    }
  }

  async function installAndRestart() {
    await relaunch();
  }

  async function saveOpenAiKey() {
    try {
      error = "";
      await invoke("set_openai_api_key", { apiKey: openaiApiKey });
      openaiApiKey = "";
      await refresh();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function clearOpenAiKey() {
    try {
      error = "";
      await invoke("clear_openai_api_key");
      await refresh();
    } catch (cause) {
      error = String(cause);
    }
  }

  async function applyPetCommand(command: string, args: Record<string, unknown> = {}) {
    if (!dashboard) return;
    try {
      error = "";
      dashboard.pet = await invoke<PetState>(command, args);
      await refresh();
    } catch (cause) {
      error = String(cause);
    }
  }

  function owns(item: ShopItem): boolean {
    return dashboard?.pet.ownedItems.includes(item.id) ?? false;
  }

  function furniturePlacement(itemId: string): number {
    return dashboard?.pet.furniture.find((item) => item.itemId === itemId)?.x ?? 0.5;
  }

  onMount(() => {
    void refresh();
    const unlisten = listen<boolean>("tracking_changed", (event) => {
      if (dashboard) {
        dashboard.settings.trackingPaused = event.payload;
      }
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<main>
  <header>
    <div>
      <p class="eyebrow">Tokengochi</p>
      <h1>Dashboard</h1>
    </div>
    <button class="ghost" onclick={refresh} disabled={busy}>Refresh</button>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if dashboard}
    {#if !dashboard.settings.onboardingComplete}
      <section class="panel onboarding">
        <div>
          <p class="eyebrow">Welcome</p>
          <h2>Pick your starter egg</h2>
        </div>
        <div class="egg-row" role="radiogroup" aria-label="Starter egg">
          {#each starterEggs as egg}
            <button
              class:selected={selectedEgg === egg.id}
              class="egg"
              style={`--tone:${egg.tone}`}
              onclick={() => (selectedEgg = egg.id)}
            >
              <span></span>
              {egg.label}
            </button>
          {/each}
        </div>
        <div class="detect">
          <span class:online={dashboard.providers.claudeCodeDetected}></span>
          Claude Code {dashboard.providers.claudeCodeDetected ? "detected" : "not detected yet"}
        </div>
        <button class="primary" onclick={finishOnboarding} disabled={busy}>Start</button>
      </section>
    {:else}
      <section class="hero panel">
        <div>
          <p class="eyebrow">Today</p>
          <strong>{dashboard.stats.food.today}</strong>
          <span>Food earned</span>
        </div>
        <div class="meter" aria-label="Progress to next Food">
          <span style={`width:${Math.round(dashboard.pet.meterProgress * 100)}%`}></span>
        </div>
        <p>
          {Math.round(dashboard.pet.meterProgress * 100)}% to next Food ·
          {formatNumber(dashboard.pet.bankedTokensToday)} / {formatNumber(dashboard.pet.tokensPerFood)} tokens
        </p>
      </section>

      <section class="stat-grid">
        <article>
          <span>Level</span>
          <strong>{dashboard.pet.level}</strong>
          <small>{dashboard.pet.evolutionStage} · {dashboard.pet.evolutionBranch}</small>
        </article>
        <article>
          <span>Fullness</span>
          <strong>{Math.round(dashboard.pet.fullness)}%</strong>
          <small>{dashboard.pet.mood}</small>
        </article>
        <article>
          <span>Week Food</span>
          <strong>{dashboard.stats.food.week}</strong>
          <small>{dashboard.stats.streakDays} day streak</small>
        </article>
        <article>
          <span>Today Tokens</span>
          <strong>{formatNumber(dashboard.stats.todayTokens.total)}</strong>
          <small>{formatNumber(dashboard.stats.todayTokens.output)} output</small>
        </article>
        <article>
          <span>Week Tokens</span>
          <strong>{formatNumber(dashboard.stats.weekTokens.total)}</strong>
          <small>{formatNumber(dashboard.stats.weekTokens.cacheRead)} cache</small>
        </article>
        <article>
          <span>Pending Food</span>
          <strong>{dashboard.pet.pendingFood}</strong>
          <small>waiting to be eaten</small>
        </article>
        <article>
          <span>Sparks</span>
          <strong>{dashboard.pet.sparks}</strong>
          <small>{formatNumber(dashboard.pet.xp)} XP · x{dashboard.pet.xpBonusMultiplier.toFixed(1)}</small>
        </article>
      </section>

      <section class="panel shop">
        <div class="section-title">
          <div>
            <p class="eyebrow">Shop</p>
            <h2>Sparks sinks</h2>
          </div>
          <span>{dashboard.pet.ownedItems.length} owned</span>
        </div>
        <div class="shop-grid">
          {#each dashboard.shopCatalog as item}
            <article class="shop-item">
              <span>{item.kind}</span>
              <strong>{item.label}</strong>
              <small>{item.priceSparks} Sparks</small>
              {#if owns(item)}
                {#if item.kind === "Cosmetic" || item.kind === "FoodSkin" || item.kind === "Heirloom"}
                  <button class="ghost" onclick={() => applyPetCommand("equip_shop_item", { itemId: item.id })}>
                    {dashboard.pet.equippedCosmetic === item.id || dashboard.pet.equippedFoodSkin === item.id ? "Equipped" : "Equip"}
                  </button>
                {:else}
                  <label class="mini-field">
                    <span>Position</span>
                    <input
                      type="range"
                      min="5"
                      max="95"
                      value={Math.round(furniturePlacement(item.id) * 100)}
                      oninput={(event) =>
                        applyPetCommand("place_furniture", {
                          itemId: item.id,
                          x: Number(event.currentTarget.value) / 100,
                        })}
                    />
                  </label>
                {/if}
              {:else}
                <button
                  class="primary"
                  disabled={dashboard.pet.sparks < item.priceSparks || item.kind === "Heirloom"}
                  onclick={() => applyPetCommand("buy_shop_item", { itemId: item.id })}
                >
                  Buy
                </button>
              {/if}
            </article>
          {/each}
        </div>
      </section>

      <!--
      Album panel disabled: no suitable art assets yet. Remove this comment wrapper to re-enable.
      <section class="panel album-panel">
        <div class="section-title">
          <div>
            <p class="eyebrow">Album</p>
            <h2>Collection</h2>
          </div>
          <button
            class="primary"
            disabled={dashboard.pet.evolutionStage !== "Elder"}
            onclick={() => applyPetCommand("prestige_pet")}
          >
            Prestige
          </button>
        </div>
        <div class="album-grid">
          {#each dashboard.pet.albumRecords as record}
            <article class="album-card">
              <span>{record.reachedDay}</span>
              <strong>{record.stage}</strong>
              <small>{record.branch} · L{record.level} · P{record.prestigeCount}</small>
            </article>
          {/each}
        </div>
      </section>
      -->

      <section class="panel settings">
        <div class="section-title">
          <div>
            <p class="eyebrow">Settings</p>
            <h2>App controls</h2>
          </div>
          <span class:online={dashboard.providers.claudeCodeDetected}>
            Claude Code {dashboard.providers.claudeCodeDetected ? "ready" : "missing"}
          </span>
        </div>

        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.claudeCodeEnabled}
            onchange={() => patchSettings({ claudeCodeEnabled: !dashboard?.settings.claudeCodeEnabled })}
          />
          <span>Track Claude Code</span>
        </label>
        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.codexCliEnabled}
            onchange={() => patchSettings({ codexCliEnabled: !dashboard?.settings.codexCliEnabled })}
          />
          <span>Track Codex CLI {dashboard.providers.codexCliDetected ? "" : "(not detected)"}</span>
        </label>
        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.openaiEnabled}
            onchange={() => patchSettings({ openaiEnabled: !dashboard?.settings.openaiEnabled })}
          />
          <span>Track OpenAI Usage API {dashboard.providers.openaiKeyConfigured ? "" : "(key needed)"}</span>
        </label>
        <div class="field openai-key">
          <span>OpenAI key</span>
          <input
            type="password"
            autocomplete="off"
            placeholder={dashboard.providers.openaiKeyConfigured ? "Stored in keychain" : "sk-..."}
            bind:value={openaiApiKey}
          />
          <div class="button-row">
            <button class="ghost" disabled={!openaiApiKey} onclick={saveOpenAiKey}>Store</button>
            <button class="ghost" onclick={clearOpenAiKey}>Clear</button>
          </div>
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            checked={!dashboard.settings.trackingPaused}
            onchange={() => patchSettings({ trackingPaused: !dashboard ? false : !dashboard.settings.trackingPaused })}
          />
          <span>Tracking active</span>
        </label>
        <label class="toggle">
          <input type="checkbox" checked={autostart} onchange={toggleAutostart} />
          <span>Launch at sign in</span>
        </label>
        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.waylandFallback}
            onchange={() => patchSettings({ waylandFallback: !dashboard?.settings.waylandFallback })}
          />
          <span>Wayland fallback window</span>
        </label>
        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.calmMode}
            onchange={() => patchSettings({ calmMode: !dashboard?.settings.calmMode })}
          />
          <span>Calm mode (disable climbing &amp; idle gags)</span>
        </label>

        <label class="field">
          <span>Pet size</span>
          <input
            type="range"
            min="70"
            max="160"
            value={dashboard.settings.petSize}
            oninput={(event) => patchSettings({ petSize: Number(event.currentTarget.value) })}
          />
          <small>{dashboard.settings.petSize}%</small>
        </label>

        <label class="field">
          <span>Monitor</span>
          <select
            value={dashboard.settings.monitorIndex}
            onchange={(event) => patchSettings({ monitorIndex: Number(event.currentTarget.value) })}
          >
            {#each Array.from({ length: dashboard.monitorCount }, (_, index) => index) as index}
              <option value={index}>Monitor {index + 1}</option>
            {/each}
          </select>
        </label>

        <div class="field update-row">
          <span>Updates</span>
          <span class="update-status">
            {#if updateStatus === "idle"}
              Not checked yet
            {:else if updateStatus === "checking"}
              Checking…
            {:else if updateStatus === "up-to-date"}
              You're on the latest version
            {:else if updateStatus === "downloading"}
              Downloading v{updateVersion}…
            {:else if updateStatus === "ready"}
              v{updateVersion} ready — restart to apply
            {:else if updateStatus === "error"}
              Update check failed
            {/if}
          </span>
          {#if updateStatus === "ready"}
            <button class="primary" onclick={installAndRestart}>Restart now</button>
          {:else}
            <button
              class="ghost"
              onclick={checkForUpdates}
              disabled={updateStatus === "checking" || updateStatus === "downloading"}
            >
              Check for updates
            </button>
          {/if}
        </div>
      </section>
    {/if}
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #101725;
    color: #f5f7ff;
    font: 16px Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }
  main {
    max-width: 920px;
    margin: auto;
    padding: 32px 20px 56px;
  }
  header,
  .section-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  h1,
  h2,
  p {
    margin: 0;
  }
  h1 {
    font-size: 34px;
  }
  h2 {
    font-size: 22px;
  }
  .eyebrow {
    color: #8da1c5;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: .12em;
    text-transform: uppercase;
  }
  .panel,
  article {
    background: #1a2436;
    border: 1px solid #2a3851;
    border-radius: 8px;
    padding: 20px;
  }
  .panel {
    margin-top: 18px;
  }
  .hero strong {
    color: #a7f070;
    font-size: 50px;
    line-height: 1;
  }
  .hero > div:first-child {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .meter {
    height: 10px;
    background: #0d1320;
    border-radius: 8px;
    margin: 18px 0 10px;
    overflow: hidden;
  }
  .meter span {
    display: block;
    height: 100%;
    background: #a7f070;
  }
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    margin-top: 18px;
  }
  article {
    display: grid;
    gap: 5px;
  }
  article strong {
    font-size: 27px;
  }
  article span,
  article small,
  .hero p,
  .field small {
    color: #aebbd1;
  }
  button,
  select {
    border: 0;
    border-radius: 8px;
    font: inherit;
  }
  button {
    cursor: pointer;
    font-weight: 800;
  }
  button:disabled {
    cursor: wait;
    opacity: .7;
  }
  .primary,
  .ghost {
    padding: 10px 15px;
  }
  .primary {
    background: #a7f070;
    color: #152016;
  }
  .ghost {
    background: #223049;
    color: #f5f7ff;
  }
  .onboarding {
    display: grid;
    gap: 18px;
  }
  .egg-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }
  .egg {
    background: #121b2b;
    border: 1px solid #2a3851;
    color: #f5f7ff;
    display: grid;
    gap: 10px;
    justify-items: center;
    padding: 16px;
  }
  .egg.selected {
    border-color: var(--tone);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--tone), transparent 70%);
  }
  .egg span {
    width: 42px;
    height: 52px;
    border-radius: 50% 50% 46% 46%;
    background: var(--tone);
  }
  .detect,
  .section-title > span {
    color: #aebbd1;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .detect span,
  .section-title > span::before {
    background: #ef7d57;
    border-radius: 999px;
    content: "";
    display: block;
    height: 9px;
    width: 9px;
  }
  .detect span.online,
  .section-title > span.online::before {
    background: #a7f070;
  }
  .settings {
    display: grid;
    gap: 12px;
  }
  .shop,
  .album-panel {
    display: grid;
    gap: 14px;
  }
  .shop-grid,
  .album-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .shop-item,
  .album-card {
    min-height: 0;
  }
  .shop-item button {
    margin-top: 6px;
  }
  .mini-field {
    display: grid;
    gap: 6px;
    margin-top: 6px;
  }
  .mini-field span {
    color: #aebbd1;
    font-size: 12px;
  }
  .toggle,
  .field {
    align-items: center;
    background: #121b2b;
    border: 1px solid #26354d;
    border-radius: 8px;
    display: grid;
    gap: 12px;
    min-height: 48px;
    padding: 10px 12px;
  }
  .toggle {
    grid-template-columns: auto 1fr;
  }
  .field {
    grid-template-columns: 120px 1fr auto;
  }
  .update-row {
    grid-template-columns: 120px 1fr auto;
  }
  .openai-key {
    grid-template-columns: 120px 1fr auto;
  }
  .button-row {
    display: flex;
    gap: 8px;
  }
  .update-status {
    color: #aebbd1;
    font-size: 14px;
  }
  input,
  select {
    accent-color: #a7f070;
  }
  select {
    background: #0d1320;
    color: #f5f7ff;
    padding: 9px 11px;
  }
  .error {
    background: #3a1d25;
    border: 1px solid #9c4656;
    border-radius: 8px;
    color: #ffb9b1;
    margin-top: 18px;
    padding: 12px;
  }
  @media (max-width: 680px) {
    header,
    .section-title {
      align-items: flex-start;
      flex-direction: column;
    }
    .stat-grid,
    .egg-row,
    .shop-grid,
    .album-grid {
      grid-template-columns: 1fr;
    }
    .field {
      grid-template-columns: 1fr;
    }
  }
</style>
