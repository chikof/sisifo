<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Browser from "./views/Browser.svelte";
	import Publish from "./views/Publish.svelte";
	import Sites from "./views/Sites.svelte";
	import NodeStats from "./views/NodeStats.svelte";

	interface Tab {
		id: string;
		title: string;
		address: string;
		gatewayUrl: string;
		inputValue: string;
		loading: boolean;
		error: string;
		history: string[];
		historyIndex: number;
		groupId: string | null;
	}
	interface TabGroup {
		id: string;
		name: string;
		color: string;
		collapsed: boolean;
	}

	function uid() {
		return Math.random().toString(36).slice(2, 9);
	}
	function newTab(address = "", groupId: string | null = null): Tab {
		return {
			id: uid(),
			title: address ? address.slice(0, 20) + "…" : "New tab",
			address,
			gatewayUrl: "",
			inputValue: address,
			loading: false,
			error: "",
			history: address ? [address] : [],
			historyIndex: address ? 0 : -1,
			groupId,
		};
	}

	let tabs: Tab[] = [newTab()];
	let activeTabId: string = tabs[0].id;
	let groups: TabGroup[] = [];

	// null = no panel open (full browser), string = panel open
	type Panel = "publish" | "sites" | "stats" | null;
	let activePanel: Panel = null;

	function togglePanel(panel: Panel) {
		activePanel = activePanel === panel ? null : panel;
	}

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

	const tools: { id: Panel; label: string; icon: string }[] = [
		{
			id: "publish",
			label: "Publish site",
			icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>`,
		},
		{
			id: "sites",
			label: "Hosted sites",
			icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
		},
		{
			id: "stats",
			label: "Node stats",
			icon: `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/></svg>`,
		},
	];

	function closePanel() {
		activePanel = null;
	}

	// Close panel on Escape
	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Escape" && activePanel) closePanel();
	}
</script>

<svelte:window on:keydown={onKeydown} />

<div class="app">
	<!-- ICON RAIL -->
	<aside class="rail">
		<div class="rail-logo" title="Sísifo">
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M12 2L2 7l10 5 10-5-10-5z" /><path
					d="M2 17l10 5 10-5"
				/><path d="M2 12l10 5 10-5" />
			</svg>
		</div>

		<div class="rail-tools">
			{#each tools as tool}
				<button
					class="rail-btn"
					class:active={activePanel === tool.id}
					on:click={() => togglePanel(tool.id)}
					title={tool.label}
					aria-label={tool.label}
				>
					{@html tool.icon}
					{#if activePanel === tool.id}
						<span class="rail-active-bar"></span>
					{/if}
				</button>
			{/each}
		</div>

		<div class="rail-footer">
			<button
				class="rail-btn daemon-btn"
				class:online={daemonRunning}
				title={daemonRunning
					? "sisid running"
					: "sisid offline — click for node stats"}
				on:click={() => togglePanel("stats")}
				aria-label="Node status"
			>
				<span class="daemon-dot" class:online={daemonRunning}></span>
			</button>
		</div>
	</aside>

	<!-- MAIN AREA: browser always mounted -->
	<div class="main" class:panel-open={activePanel !== null}>
		<div class="browser-wrap">
			<Browser bind:tabs bind:activeTabId bind:groups />
		</div>

		<!-- PANEL OVERLAY -->
		{#if activePanel}
			<div
				class="panel-backdrop"
				on:click={closePanel}
				aria-hidden="true"
			></div>
			<div class="panel" role="dialog" aria-label={activePanel}>
				<div class="panel-header">
					<span class="panel-title">
						{activePanel === "publish"
							? "Publish site"
							: activePanel === "sites"
								? "Hosted sites"
								: "Node"}
					</span>
					<button
						class="panel-close"
						on:click={closePanel}
						aria-label="Close panel"
					>
						<svg
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2.5"
							stroke-linecap="round"
							><line x1="18" y1="6" x2="6" y2="18" /><line
								x1="6"
								y1="6"
								x2="18"
								y2="18"
							/></svg
						>
					</button>
				</div>
				<div class="panel-body">
					{#if activePanel === "publish"}
						<Publish />
					{:else if activePanel === "sites"}
						<Sites />
					{:else if activePanel === "stats"}
						<NodeStats />
					{/if}
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	:global(*, *::before, *::after) {
		box-sizing: border-box;
		margin: 0;
		padding: 0;
	}
	:global(body) {
		background: #f5f4f2;
		color: #1a1a1a;
		font-family: "DM Sans", sans-serif;
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
		--mono: "DM Mono", monospace;
		--radius: 6px;
		--rail-w: 48px;
	}

	.app {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}

	/* ── ICON RAIL ── */
	.rail {
		width: var(--rail-w);
		min-width: var(--rail-w);
		background: var(--surface);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 8px 0;
		gap: 2px;
		user-select: none;
		z-index: 10;
	}

	.rail-logo {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text);
		margin-bottom: 8px;
		border-radius: 6px;
	}

	.rail-tools {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		align-items: center;
	}

	.rail-btn {
		position: relative;
		width: 36px;
		height: 36px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 8px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.rail-btn:hover {
		background: var(--surface-2);
		color: var(--text-2);
	}
	.rail-btn.active {
		background: var(--accent-light);
		color: var(--accent);
	}

	.rail-active-bar {
		position: absolute;
		left: 0;
		top: 50%;
		transform: translateY(-50%);
		width: 2px;
		height: 18px;
		background: var(--accent);
		border-radius: 0 2px 2px 0;
	}

	.rail-footer {
		margin-top: auto;
		padding-bottom: 4px;
	}

	.daemon-btn {
		width: 36px;
		height: 36px;
	}
	.daemon-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-3);
		transition: background 0.3s;
	}
	.daemon-dot.online {
		background: var(--accent);
		box-shadow: 0 0 0 2px var(--accent-light);
		animation: pulse 2s ease-in-out infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	/* ── MAIN ── */
	.main {
		flex: 1;
		position: relative;
		overflow: hidden;
		display: flex;
	}

	.browser-wrap {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		transition: margin-right 0.2s ease;
	}

	/* ── PANEL ── */
	.panel-backdrop {
		position: absolute;
		inset: 0;
		z-index: 19;
		background: transparent;
	}

	.panel {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		width: 420px;
		max-width: 90%;
		background: var(--surface);
		border-left: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		z-index: 20;
		animation: slideIn 0.18s ease;
		box-shadow: -4px 0 24px rgba(0, 0, 0, 0.06);
	}

	@keyframes slideIn {
		from {
			transform: translateX(20px);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 14px 20px;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.panel-title {
		font-size: 13.5px;
		font-weight: 600;
		color: var(--text);
		letter-spacing: -0.01em;
	}

	.panel-close {
		width: 26px;
		height: 26px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 4px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.panel-close:hover {
		background: var(--surface-2);
		color: var(--text);
	}

	.panel-body {
		flex: 1;
		overflow-y: auto;
	}
</style>
