<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import { check, type Update } from "@tauri-apps/plugin-updater";
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
    furniture: { itemId: string; x: number; visible: boolean }[];
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
    agentStatusNotificationsEnabled: boolean;
  };
  type TokenTotals = { input: number; output: number; cacheRead: number; total: number };
  // Per-provider breakdown keyed by provider id ("claude_code", "codex_cli", ...).
  // Providers with no usage in the window are absent from the map.
  type TokensByProvider = Record<string, TokenTotals>;
  type FoodStats = { today: number; week: number };
  type ShopItem = { id: string; label: string; kind: "Cosmetic" | "FoodSkin" | "Furniture"; priceSparks: number };
  type DashboardState = {
    pet: PetState;
    settings: AppSettings;
    providers: { claudeCodeDetected: boolean; codexCliDetected: boolean; openaiKeyConfigured: boolean };
    stats: {
      food: FoodStats;
      todayTokens: TokenTotals;
      weekTokens: TokenTotals;
      todayTokensByProvider: TokensByProvider;
      weekTokensByProvider: TokensByProvider;
      streakDays: number;
    };
    monitorCount: number;
    shopCatalog: ShopItem[];
  };

  const ZERO_TOTALS: TokenTotals = { input: 0, output: 0, cacheRead: 0, total: 0 };

  // Providers shown as their own token cards, in display order. OpenAI/manual
  // usage still counts toward the combined totals but isn't broken out here.
  const TOKEN_PROVIDERS = [
    { id: "claude_code", label: "Claude" },
    { id: "codex_cli", label: "Codex" },
  ] as const;

  function providerTotals(map: TokensByProvider | undefined, id: string): TokenTotals {
    return map?.[id] ?? ZERO_TOTALS;
  }

  const starterEggs = [
    { id: "sprout", label: "Sprout", tone: "#a7f070" },
    { id: "ember", label: "Ember", tone: "#ef7d57" },
    { id: "bubble", label: "Bubble", tone: "#73eff7" },
  ];
  // Keep the starter picker code around for future multi-pet selection. For
  // now there is only one pet, so onboarding silently uses the default
  // "sprout" starter and skips the choice UI.
  const SHOW_STARTER_EGG_PICKER = false;

  type UpdateStatus = "idle" | "checking" | "up-to-date" | "available" | "downloading" | "ready" | "error";

  let dashboard = $state<DashboardState | null>(null);
  let autostart = $state(false);
  let selectedEgg = $state("sprout");
  let busy = $state(false);
  let error = $state("");
  let updateStatus = $state<UpdateStatus>("idle");
  let updateVersion = $state("");
  let currentVersion = $state("");
  // Holds the checked update handle so the "Download & install" action can
  // reuse it instead of calling check() again.
  let pendingUpdate: Update | null = null;
  let openaiApiKey = $state("");
  type AgentStatusHookStatus = { installed: boolean; settingsPath: string };
  let agentStatusHookStatus = $state<AgentStatusHookStatus | null>(null);
  let agentStatusHookCheckError = $state("");
  let agentStatusHookBusy = $state(false);
  // Codex CLI counterpart of the above (task 0027) - same shape, separate
  // state since the two providers install/uninstall independently.
  let codexHookStatus = $state<AgentStatusHookStatus | null>(null);
  let codexHookCheckError = $state("");
  let codexHookBusy = $state(false);

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
    // Kept out of the Promise.all above: a malformed ~/.claude/settings.json
    // should only degrade this one status readout, not the whole dashboard.
    try {
      agentStatusHookCheckError = "";
      agentStatusHookStatus = await invoke<AgentStatusHookStatus>("agent_status_hook_status");
    } catch (cause) {
      agentStatusHookStatus = null;
      agentStatusHookCheckError = String(cause);
    }
    // Same rationale as above: a malformed ~/.codex/hooks.json should only
    // degrade this readout, not the whole dashboard.
    try {
      codexHookCheckError = "";
      codexHookStatus = await invoke<AgentStatusHookStatus>("codex_hook_status");
    } catch (cause) {
      codexHookStatus = null;
      codexHookCheckError = String(cause);
    }
  }

  async function installAgentStatusHooks() {
    try {
      agentStatusHookBusy = true;
      agentStatusHookCheckError = "";
      await invoke("install_agent_status_hooks");
      agentStatusHookStatus = await invoke<AgentStatusHookStatus>("agent_status_hook_status");
    } catch (cause) {
      agentStatusHookCheckError = String(cause);
    } finally {
      agentStatusHookBusy = false;
    }
  }

  async function uninstallAgentStatusHooks() {
    try {
      agentStatusHookBusy = true;
      agentStatusHookCheckError = "";
      await invoke("uninstall_agent_status_hooks");
      agentStatusHookStatus = await invoke<AgentStatusHookStatus>("agent_status_hook_status");
    } catch (cause) {
      agentStatusHookCheckError = String(cause);
    } finally {
      agentStatusHookBusy = false;
    }
  }

  async function installCodexHooks() {
    try {
      codexHookBusy = true;
      codexHookCheckError = "";
      await invoke("install_codex_hooks");
      codexHookStatus = await invoke<AgentStatusHookStatus>("codex_hook_status");
    } catch (cause) {
      codexHookCheckError = String(cause);
    } finally {
      codexHookBusy = false;
    }
  }

  async function uninstallCodexHooks() {
    try {
      codexHookBusy = true;
      codexHookCheckError = "";
      await invoke("uninstall_codex_hooks");
      codexHookStatus = await invoke<AgentStatusHookStatus>("codex_hook_status");
    } catch (cause) {
      codexHookCheckError = String(cause);
    } finally {
      codexHookBusy = false;
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

  async function checkForUpdates(auto = false) {
    try {
      updateStatus = "checking";
      if (!auto) error = "";
      const update = await check();
      if (!update) {
        updateStatus = "up-to-date";
        return;
      }
      pendingUpdate = update;
      updateVersion = update.version;
      updateStatus = "available";
    } catch (cause) {
      // A background auto-check failing (e.g. offline on startup) shouldn't
      // surface an alarming error banner - only report failures from an
      // explicit, user-initiated check.
      updateStatus = auto ? "idle" : "error";
      if (!auto) error = String(cause);
    }
  }

  async function downloadAndInstallUpdate() {
    if (!pendingUpdate) return;
    try {
      updateStatus = "downloading";
      error = "";
      await pendingUpdate.downloadAndInstall();
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

  const ITEM_PREVIEWS: Record<string, { src: string; alt: string }> = {
    "hat-leaf": { src: "/overlay/sprites/items/hat-leaf-sprite-32x32.png", alt: "Leaf Cap pixel-art item" },
    "hat-mushroom": { src: "/overlay/sprites/items/hat-mushroom-sprite-32x32.png", alt: "Mushroom Cap pixel-art item" },
    "food-sushi": { src: "/overlay/sprites/items/food-sushi-sprite-32x32.png", alt: "Sushi Food pixel-art item" },
    "food-banh-mi": { src: "/overlay/sprites/items/food-banh-mi-sprite-32x32.png", alt: "Banh Mi Food pixel-art item" },
    "furniture-bed": { src: "/overlay/sprites/items/furniture-bed-sprite-80x40.png", alt: "Tiny Bed pixel-art item" },
    "furniture-plant": { src: "/overlay/sprites/items/furniture-plant-sprite-80x40.png", alt: "Desk Plant pixel-art item" },
  };
  function itemPreview(item: ShopItem) {
    return ITEM_PREVIEWS[item.id];
  }

  function furniturePlacement(itemId: string): number {
    return dashboard?.pet.furniture.find((item) => item.itemId === itemId)?.x ?? 0.5;
  }

  function furnitureVisible(itemId: string): boolean {
    return dashboard?.pet.furniture.find((item) => item.itemId === itemId)?.visible ?? true;
  }

  onMount(() => {
    void refresh();
    void getVersion().then((version) => {
      currentVersion = version;
    });
    void checkForUpdates(true);
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
    <div class="header-actions">
      {#if updateStatus === "available"}
        <button class="update-badge" onclick={downloadAndInstallUpdate}>
          v{updateVersion} available
        </button>
      {:else if updateStatus === "downloading"}
        <span class="update-badge is-busy">Downloading v{updateVersion}…</span>
      {:else if updateStatus === "ready"}
        <button class="update-badge is-ready" onclick={installAndRestart}>
          v{updateVersion} ready — restart
        </button>
      {/if}
      <button class="ghost" onclick={refresh} disabled={busy}>Refresh</button>
    </div>
  </header>

  <div class="scroll-area">
  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if dashboard}
    {#if !dashboard.settings.onboardingComplete}
      <section class="panel onboarding">
        <div>
          <p class="eyebrow">Welcome</p>
          <h2>Start Tokengochi</h2>
        </div>
        {#if SHOW_STARTER_EGG_PICKER}
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
        {/if}
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
        {#each TOKEN_PROVIDERS as provider}
          <article>
            <span>{provider.label} Tokens</span>
            <strong>{formatNumber(providerTotals(dashboard.stats.todayTokensByProvider, provider.id).total)}</strong>
            <small>today · {formatNumber(providerTotals(dashboard.stats.weekTokensByProvider, provider.id).total)} this week</small>
          </article>
        {/each}
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
              {#if itemPreview(item)}
                <img class="item-preview" src={itemPreview(item).src} alt={itemPreview(item).alt} width="56" height="56" />
              {/if}
              <span>{item.kind}</span>
              <strong>{item.label}</strong>
              <small>{item.priceSparks} Sparks</small>
              {#if owns(item)}
                {#if item.kind === "Cosmetic" || item.kind === "FoodSkin"}
                  <button class="ghost" onclick={() => applyPetCommand("equip_shop_item", { itemId: item.id })}>
                    {dashboard.pet.equippedCosmetic === item.id || dashboard.pet.equippedFoodSkin === item.id ? "Unequip" : "Equip"}
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
                  <button class="ghost" onclick={() => applyPetCommand("toggle_furniture_visibility", { itemId: item.id })}>
                    {furnitureVisible(item.id) ? "Hide" : "Show"}
                  </button>
                {/if}
              {:else}
                <button
                  class="primary"
                  disabled={dashboard.pet.sparks < item.priceSparks}
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
        <label class="toggle">
          <input
            type="checkbox"
            checked={dashboard.settings.agentStatusNotificationsEnabled}
            onchange={() =>
              patchSettings({
                agentStatusNotificationsEnabled: !dashboard?.settings.agentStatusNotificationsEnabled,
              })}
          />
          <span>Pet reacts to agent status (done / needs approval)</span>
        </label>
        <div class="field agent-status-hook">
          <span class="label-with-hint">
            Claude Code hook
            <button type="button" class="info-hint" aria-label="What does the Claude Code hook do?">
              <span aria-hidden="true">?</span>
              <span class="tooltip-text" role="tooltip">
                Adds a `Stop`/`Notification` hook to your global
                <code>~/.claude/settings.json</code> so Claude Code tells Tokengochi
                when a turn finishes or needs your approval - that's what powers the
                "Pet reacts to agent status" toggle above. Only a session id is
                read from the hook payload; no prompt or file content ever leaves
                your machine. Safe to install more than once (it won't duplicate
                itself) and never touches your other hooks.
              </span>
            </button>
          </span>
          {#if agentStatusHookStatus?.installed}
            <span class="hook-status hook-status-ok">Installed in {agentStatusHookStatus.settingsPath}</span>
            <div class="button-row">
              <button class="ghost" disabled={agentStatusHookBusy} onclick={uninstallAgentStatusHooks}>
                {agentStatusHookBusy ? "Removing…" : "Remove hook"}
              </button>
            </div>
          {:else}
            <span class="hook-status">
              Not installed - the pet can't react to Claude turns until this is set up globally.
            </span>
            <div class="button-row">
              <button class="primary" disabled={agentStatusHookBusy} onclick={installAgentStatusHooks}>
                {agentStatusHookBusy ? "Installing…" : "Install hook"}
              </button>
            </div>
          {/if}
          {#if agentStatusHookCheckError}
            <small class="error">{agentStatusHookCheckError}</small>
          {/if}
        </div>

        <div class="field agent-status-hook">
          <span class="label-with-hint">
            Codex CLI hook
            <button type="button" class="info-hint" aria-label="What does the Codex CLI hook do?">
              <span aria-hidden="true">?</span>
              <span class="tooltip-text" role="tooltip">
                Adds a `Stop`/`PermissionRequest`/`PostToolUse` hook to your global
                <code>~/.codex/hooks.json</code> so Codex CLI tells Tokengochi when a
                turn finishes or needs your approval - same reaction as the Claude
                Code hook above. Only a session id is read from the hook payload; no
                prompt or file content ever leaves your machine. Safe to install more
                than once (it won't duplicate itself) and never touches your other
                hooks. Requires a Codex CLI version with hooks support.
              </span>
            </button>
          </span>
          {#if codexHookStatus?.installed}
            <span class="hook-status hook-status-ok">Installed in {codexHookStatus.settingsPath}</span>
            <div class="button-row">
              <button class="ghost" disabled={codexHookBusy} onclick={uninstallCodexHooks}>
                {codexHookBusy ? "Removing…" : "Remove hook"}
              </button>
            </div>
          {:else}
            <span class="hook-status">
              Not installed - the pet can't react to Codex turns until this is set up globally.
            </span>
            <div class="button-row">
              <button class="primary" disabled={codexHookBusy} onclick={installCodexHooks}>
                {codexHookBusy ? "Installing…" : "Install hook"}
              </button>
            </div>
          {/if}
          {#if codexHookCheckError}
            <small class="error">{codexHookCheckError}</small>
          {/if}
        </div>

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
          <span>Updates ({currentVersion ? `v${currentVersion}` : "…"})</span>
          <span class="update-status">
            {#if updateStatus === "idle"}
              Not checked yet
            {:else if updateStatus === "checking"}
              Checking…
            {:else if updateStatus === "up-to-date"}
              You're on the latest version
            {:else if updateStatus === "available"}
              v{updateVersion} available
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
          {:else if updateStatus === "available"}
            <button class="primary" onclick={downloadAndInstallUpdate}>Download &amp; install</button>
          {:else}
            <button
              class="ghost"
              onclick={() => checkForUpdates(false)}
              disabled={updateStatus === "checking" || updateStatus === "downloading"}
            >
              Check for updates
            </button>
          {/if}
        </div>
      </section>
    {/if}
  {/if}
  </div>
</main>

<style>
  :global(html),
  :global(body) {
    height: 100%;
    overflow: hidden;
  }
  :global(html) {
    background: #101725;
  }
  :global(body) {
    margin: 0;
    background: #101725;
    color: #f5f7ff;
    font: 16px Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }
  main {
    max-width: 920px;
    margin: auto;
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 0 20px;
  }
  header,
  .section-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  header {
    flex: 0 0 auto;
    background: #101725;
    padding: 32px 0 12px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .update-badge {
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 800;
    background: #2a3851;
    color: #a7f070;
    border: 1px solid #3a4d70;
  }
  .update-badge.is-busy {
    color: #aebbd1;
    cursor: default;
  }
  .update-badge.is-ready {
    background: #a7f070;
    color: #152016;
    border-color: #a7f070;
  }
  .scroll-area {
    flex: 1 1 auto;
    overflow-y: auto;
    padding-bottom: 56px;
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
  .shop {
    display: grid;
    gap: 14px;
  }
  .shop-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
  .shop-item {
    min-height: 0;
  }
  .shop-item button {
    margin-top: 6px;
  }
  .item-preview {
    background: #0c1220;
    border: 1px solid #26354d;
    border-radius: 8px;
    display: block;
    image-rendering: pixelated;
    margin-bottom: 4px;
    object-fit: contain;
    padding: 6px;
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
  .agent-status-hook {
    grid-template-columns: 120px 1fr auto;
  }
  .hook-status {
    color: #aebbd1;
    font-size: 14px;
  }
  .hook-status-ok {
    color: #a7f070;
  }
  .agent-status-hook small.error {
    grid-column: 1 / -1;
    margin-top: 0;
  }
  .label-with-hint {
    align-items: center;
    display: inline-flex;
    gap: 6px;
  }
  .info-hint {
    background: none;
    border: 0;
    color: #aebbd1;
    cursor: help;
    display: inline-flex;
    padding: 0;
    position: relative;
  }
  .info-hint > span[aria-hidden] {
    align-items: center;
    background: #223049;
    border-radius: 999px;
    display: inline-flex;
    font-size: 11px;
    font-weight: 800;
    height: 15px;
    justify-content: center;
    width: 15px;
  }
  .info-hint .tooltip-text {
    background: #0d1320;
    border: 1px solid #2a3851;
    border-radius: 8px;
    bottom: calc(100% + 8px);
    color: #f5f7ff;
    font-size: 12px;
    font-weight: 400;
    left: 0;
    line-height: 1.5;
    opacity: 0;
    padding: 10px 12px;
    pointer-events: none;
    position: absolute;
    transition: opacity 0.12s ease;
    visibility: hidden;
    width: 260px;
    z-index: 5;
  }
  .info-hint .tooltip-text code {
    background: #1a2334;
    border-radius: 4px;
    padding: 1px 4px;
  }
  .info-hint:hover .tooltip-text,
  .info-hint:focus-visible .tooltip-text,
  .info-hint:focus .tooltip-text {
    opacity: 1;
    visibility: visible;
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
    .shop-grid {
      grid-template-columns: 1fr;
    }
    .field {
      grid-template-columns: 1fr;
    }
  }
</style>
