<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type EconomyConfig = {
    tokens_per_food: number;
    weight_output: number;
    weight_input: number;
    weight_cache_read: number;
    daily_soft_cap: number;
    soft_cap_escalation: number;
    daily_hard_cap: number;
    pantry_max: number;
    fullness_per_food: number;
    fullness_decay_per_24h: number;
    xp_per_food: number;
    xp_curve_base: number;
    xp_curve_exponent: number;
  };

  let config = $state<EconomyConfig | null>(null);
  let error = $state("");

  onMount(async () => {
    try {
      config = await invoke<EconomyConfig>("get_config");
    } catch (e) {
      error = String(e);
    }
  });
</script>

<main class="container">
  <h1>Tokengochi</h1>
  <p>Dashboard shell - stats, settings, album, and shop land here.</p>

  {#if error}
    <p class="error">Failed to load economy.toml: {error}</p>
  {:else if config}
    <h2>Economy config (from economy.toml)</h2>
    <table>
      <tbody>
        {#each Object.entries(config) as [key, value]}
          <tr>
            <td>{key}</td>
            <td>{value}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p>Loading config...</p>
  {/if}
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  .container {
    margin: 0 auto;
    max-width: 640px;
    padding: 2rem 1rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  td {
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid #ddd;
    font-variant-numeric: tabular-nums;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
    td {
      border-bottom-color: #444;
    }
  }
</style>
