<script lang="ts">
import { onMount } from "svelte";
import { invoke } from "@tauri-apps/api/core";
import Browser from "./views/Browser.svelte";
import Publish from "./views/Publish.svelte";
import Sites from "./views/Sites.svelte";
import NodeStats from "./views/NodeStats.svelte";

type View = "browser" | "publish" | "sites" | "stats";
let activeView: View = "browser";

let daemonRunning = false;
let nodeId = "";

onMount(async () => {
	try {
		daemonRunning = await invoke<boolean>("daemon_running");
		nodeId = await invoke<string>("node_identity");
	} catch (e) {
		console.error(e);
	}
});

const navItems: { id: View; label: string; icon: string }[] = [
	{
		id: "browser",
		label: "Browse",
		icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>`,
	},
	{
		id: "publish",
		label: "Publish",
		icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>`,
	},
	{
		id: "sites",
		label: "Hosted",
		icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
	},
	{
		id: "stats",
		label: "Node",
		icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/></svg>`,
	},
];
</script>

<div class="app">
  <aside class="sidebar">
    <div class="logo">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
      </svg>
      <span>sisi</span>
    </div>

    <nav>
      {#each navItems as item}
        <button
          class="nav-item"
          class:active={activeView === item.id}
          on:click={() => (activeView = item.id)}
          title={item.label}
        >
          {@html item.icon}
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>

    <div class="sidebar-footer">
      <div class="daemon-indicator" class:online={daemonRunning} title={daemonRunning ? 'sisid running' : 'sisid offline'}>
        <span class="dot"></span>
        <span>{daemonRunning ? 'daemon on' : 'daemon off'}</span>
      </div>
      {#if nodeId}
        <div class="node-id" title={nodeId}>
          {nodeId.slice(0, 8)}…
        </div>
      {/if}
    </div>
  </aside>

  <main class="content">
    {#if activeView === 'browser'}
      <Browser />
    {:else if activeView === 'publish'}
      <Publish />
    {:else if activeView === 'sites'}
      <Sites />
    {:else if activeView === 'stats'}
      <NodeStats />
    {/if}
  </main>
</div>

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #f5f4f2;
    color: #1a1a1a;
    font-family: 'DM Sans', sans-serif;
    font-size: 14px;
    -webkit-font-smoothing: antialiased;
    height: 100vh;
    overflow: hidden;
  }
  :global(:root) {
    --accent: #0d9488;
    --accent-light: #e6f4f3;
    --border: #e2e0dc;
    --surface: #ffffff;
    --surface-2: #f0ede9;
    --text: #1a1a1a;
    --text-2: #6b6760;
    --text-3: #a09d99;
    --mono: 'DM Mono', monospace;
    --radius: 6px;
    --sidebar-w: 160px;
  }

  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .sidebar {
    width: var(--sidebar-w);
    min-width: var(--sidebar-w);
    background: var(--surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 16px 12px;
    gap: 4px;
    user-select: none;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 16px;
    color: var(--text);
    font-weight: 600;
    font-size: 15px;
    letter-spacing: -0.02em;
    border-bottom: 1px solid var(--border);
    margin-bottom: 8px;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border: none;
    background: none;
    color: var(--text-2);
    font-family: 'DM Sans', sans-serif;
    font-size: 13.5px;
    border-radius: var(--radius);
    cursor: pointer;
    width: 100%;
    text-align: left;
    transition: background 0.12s, color 0.12s;
  }
  .nav-item:hover { background: var(--surface-2); color: var(--text); }
  .nav-item.active { background: var(--accent-light); color: var(--accent); font-weight: 500; }

  .sidebar-footer {
    border-top: 1px solid var(--border);
    padding-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .daemon-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-3);
    padding: 4px 8px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-3);
    flex-shrink: 0;
  }
  .daemon-indicator.online .dot { background: var(--accent); }
  .daemon-indicator.online { color: var(--accent); }

  .node-id {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--text-3);
    padding: 2px 8px;
    letter-spacing: 0.02em;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
