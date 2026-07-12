<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type PetState = {
    fullness: number;
    mood: string;
    xp: number;
    level: number;
    pendingFood: number;
    pantry: number;
    foodEarnedToday: number;
    bankedTokensToday: number;
    tokensPerFood: number;
    meterProgress: number;
  };
  type AppSettings = {
    onboardingComplete: boolean;
    starterEgg: string;
    claudeCodeEnabled: boolean;
    petSize: number;
    monitorIndex: number;
    waylandFallback: boolean;
    trackingPaused: boolean;
  };
  type TokenTotals = { input: number; output: number; cacheRead: number; total: number };
  type FoodStats = { today: number; week: number };
  type DashboardState = {
    pet: PetState;
    settings: AppSettings;
    providers: { claudeCodeDetected: boolean };
    stats: { food: FoodStats; todayTokens: TokenTotals; weekTokens: TokenTotals; streakDays: number };
    monitorCount: number;
  };

  const starterEggs = [
    { id: "sprout", label: "Sprout", tone: "#a7f070" },
    { id: "ember", label: "Ember", tone: "#ef7d57" },
    { id: "bubble", label: "Bubble", tone: "#73eff7" },
  ];

  let dashboard = $state<DashboardState | null>(null);
  let autostart = $state(false);
  let selectedEgg = $state("sprout");
  let busy = $state(false);
  let error = $state("");

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
          <small>{formatNumber(dashboard.pet.xp)} XP</small>
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
          <span>Pantry</span>
          <strong>{dashboard.pet.pantry}</strong>
          <small>{dashboard.pet.pendingFood} waiting</small>
        </article>
      </section>

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
    .egg-row {
      grid-template-columns: 1fr;
    }
    .field {
      grid-template-columns: 1fr;
    }
  }
</style>
