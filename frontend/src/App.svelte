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
		iconUrl: string | null;
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
			iconUrl: null,
		};
	}

	let tabs = $state<Tab[]>([newTab()]);
	// svelte-ignore state_referenced_locally
	let activeTabId = $state<string>(tabs[0].id);
	let groups = $state<TabGroup[]>([]);

	type ModalView = "publish" | "sites" | "stats";

	let modalOpen = $state(false);
	let activeView = $state<ModalView>("stats");

	// Component refs so we can call refresh from the shared header
	let statsRef = $state<{ loadAll: () => void } | null>(null);
	let sitesRef = $state<{ load: () => void } | null>(null);
	let statsLoading = $state(false);
	let sitesLoading = $state(false);

	function openModal(view: ModalView = "stats") {
		activeView = view;
		modalOpen = true;
	}

	function closeModal() {
		modalOpen = false;
	}

	const navItems: { id: ModalView; label: string; icon: string }[] = [
		{
			id: "stats",
			label: "Node",
			icon: `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M12 1v4M12 19v4M4.22 4.22l2.83 2.83M16.95 16.95l2.83 2.83M1 12h4M19 12h4M4.22 19.78l2.83-2.83M16.95 7.05l2.83-2.83"/></svg>`,
		},
		{
			id: "sites",
			label: "Hosted sites",
			icon: `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`,
		},
		{
			id: "publish",
			label: "Publish site",
			icon: `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>`,
		},
	];

	let isDark = $state(false);

	function applyTheme(dark: boolean) {
		document.documentElement.setAttribute(
			"data-theme",
			dark ? "dark" : "light",
		);
		try {
			localStorage.setItem("sisi-theme", dark ? "dark" : "light");
		} catch {}
	}

	function toggleTheme() {
		isDark = !isDark;
		applyTheme(isDark);
	}

	let daemonRunning = $state(false);

	onMount(async () => {
		// Theme init
		try {
			const stored = localStorage.getItem("sisi-theme");
			isDark =
				stored === "dark"
					? true
					: stored === "light"
						? false
						: window.matchMedia("(prefers-color-scheme: dark)")
								.matches;
		} catch {
			isDark = false;
		}
		applyTheme(isDark);

		// Daemon status
		try {
			daemonRunning = await invoke<boolean>("daemon_running");
		} catch {}
	});

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Escape" && modalOpen) closeModal();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
	<!-- SLIM RAIL -->
	<aside class="rail">
		<!-- Logo -->
		<div class="rail-logo" title="Sísifo">
			<svg
				width="18"
				height="18"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.8"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M12 2L2 7l10 5 10-5-10-5z" />
				<path d="M2 17l10 5 10-5" />
				<path d="M2 12l10 5 10-5" />
			</svg>
		</div>

		<!-- Footer controls -->
		<div class="rail-footer">
			<!-- Theme toggle -->
			<button
				class="rail-btn"
				onclick={toggleTheme}
				title={isDark ? "Light mode" : "Dark mode"}
				aria-label="Toggle theme"
			>
				{#if isDark}
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<circle cx="12" cy="12" r="4" />
						<path
							d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
						/>
					</svg>
				{:else}
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.8"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path
							d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
						/>
					</svg>
				{/if}
			</button>

			<!-- Daemon status — opens the modal -->
			<button
				class="daemon-btn"
				onclick={() => openModal("stats")}
				title={daemonRunning
					? "sisid running · open settings"
					: "sisid offline · open settings"}
				aria-label="Open settings"
			>
				<span class={["daemon-dot", daemonRunning && "online"]}></span>
			</button>
		</div>
	</aside>

	<!-- BROWSER (full width minus rail) -->
	<div class="browser-wrap">
		<Browser bind:tabs bind:activeTabId bind:groups />
	</div>

	<!-- CENTRED MODAL -->
	{#if modalOpen}
		<div
			class="modal-backdrop"
			onclick={closeModal}
			aria-hidden="true"
		></div>

		<div
			class="modal"
			role="dialog"
			aria-modal="true"
			aria-label="Settings"
		>
			<!-- LEFT SIDEBAR -->
			<nav class="modal-nav">
				<p class="modal-nav-label">sísifo</p>

				<div class="modal-nav-items">
					{#each navItems as item (item.id)}
						<button
							class={[
								"modal-nav-item",
								activeView === item.id && "active",
							]}
							onclick={() => (activeView = item.id)}
						>
							<span class="nav-item-icon">{@html item.icon}</span>
							<span class="nav-item-label">{item.label}</span>
						</button>
					{/each}
				</div>
			</nav>

			<!-- DIVIDER -->
			<div class="modal-divider"></div>

			<!-- RIGHT CONTENT -->
			<div class="modal-content">
				<!-- Unified header: title + actions always in same place -->
				<div class="modal-header">
					<div class="modal-header-text">
						<h2 class="modal-title">
							{activeView === "stats"
								? "Node"
								: activeView === "sites"
									? "Hosted sites"
									: "Publish site"}
						</h2>
						<p class="modal-subtitle">
							{activeView === "stats"
								? "Live stats for your local iroh node."
								: activeView === "sites"
									? "Sites you're seeding on the network."
									: "Publish a folder to the network."}
						</p>
					</div>
					<div class="modal-header-actions">
						{#if activeView === "stats"}
							<button
								class="header-action-btn"
								onclick={() => statsRef?.loadAll()}
								title="Refresh"
								aria-label="Refresh"
							>
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									class={statsLoading ? "spin" : ""}
								>
									<polyline points="23 4 23 10 17 10" />
									<polyline points="1 20 1 14 7 14" />
									<path
										d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
									/>
								</svg>
							</button>
						{:else if activeView === "sites"}
							<button
								class="header-action-btn"
								onclick={() => sitesRef?.load()}
								title="Refresh"
								aria-label="Refresh"
							>
								<svg
									width="14"
									height="14"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									class={sitesLoading ? "spin" : ""}
								>
									<polyline points="23 4 23 10 17 10" />
									<polyline points="1 20 1 14 7 14" />
									<path
										d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
									/>
								</svg>
							</button>
						{/if}
						<button
							class="header-action-btn close-btn"
							onclick={closeModal}
							aria-label="Close"
						>
							<svg
								width="13"
								height="13"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.2"
								stroke-linecap="round"
							>
								<line x1="18" y1="6" x2="6" y2="18" />
								<line x1="6" y1="6" x2="18" y2="18" />
							</svg>
						</button>
					</div>
				</div>

				<div class="modal-view">
					{#if activeView === "stats"}
						<NodeStats
							bind:this={statsRef}
							bind:loading={statsLoading}
						/>
					{:else if activeView === "sites"}
						<Sites
							bind:this={sitesRef}
							bind:loading={sitesLoading}
						/>
					{:else if activeView === "publish"}
						<Publish />
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	/* ── CSS CUSTOM PROPERTIES ── */
	:global(:root) {
		--accent: #0d9488;
		--accent-dim: #0a6b61;
		--accent-light: #e6f4f3;

		--bg: #f5f4f2;
		--surface: #ffffff;
		--surface-2: #f0ede9;
		--surface-3: #e8e4de;
		--border: #e2e0dc;

		--text: #1a1a1a;
		--text-2: #6b6760;
		--text-3: #a09d99;

		--mono: "DM Mono", monospace;
		--sans: "DM Sans", "Helvetica Neue", sans-serif;
		--radius: 6px;
		--rail-w: 44px;
	}

	:global([data-theme="dark"]) {
		--accent-light: #0e2826;

		--bg: #1c1b19;
		--surface: #242320;
		--surface-2: #2a2927;
		--surface-3: #302f2c;
		--border: #353330;

		--text: #edeae5;
		--text-2: #8c8880;
		--text-3: #55524d;
	}

	:global(body) {
		background: var(--bg);
		color: var(--text);
		font-family: var(--sans);
		font-size: 14px;
		-webkit-font-smoothing: antialiased;
		height: 100vh;
		overflow: hidden;
	}

	/* ── LAYOUT ── */
	.app {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}

	.browser-wrap {
		flex: 1;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	/* ── SLIM RAIL ── */
	.rail {
		width: var(--rail-w);
		min-width: var(--rail-w);
		background: var(--surface);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 10px 0 10px;
		z-index: 10;
		user-select: none;
	}

	.rail-logo {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--accent);
		border-radius: 8px;
		flex-shrink: 0;
	}

	.rail-footer {
		margin-top: auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}

	.rail-btn {
		width: 32px;
		height: 32px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 7px;
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

	/* Daemon status button */
	.daemon-btn {
		width: 32px;
		height: 32px;
		border: none;
		background: none;
		border-radius: 7px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 0.12s;
	}
	.daemon-btn:hover {
		background: var(--surface-2);
	}

	.daemon-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-3);
		transition: background 0.3s;
		flex-shrink: 0;
	}
	.daemon-dot.online {
		background: var(--accent);
		box-shadow: 0 0 0 2px var(--accent-light);
		animation: pulse 2.5s ease-in-out infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.45;
		}
	}

	/* ── MODAL BACKDROP ── */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: 100;
		background: rgba(0, 0, 0, 0.32);
		backdrop-filter: blur(3px);
		-webkit-backdrop-filter: blur(3px);
		animation: fadeIn 0.15s ease;
	}
	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	/* ── CENTRED MODAL ── */
	.modal {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 101;

		width: min(820px, calc(100vw - 48px));
		height: min(560px, calc(100vh - 80px));

		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 14px;
		box-shadow:
			0 0 0 1px rgba(0, 0, 0, 0.04),
			0 24px 64px rgba(0, 0, 0, 0.18),
			0 8px 24px rgba(0, 0, 0, 0.1);

		display: flex;
		overflow: hidden;
		animation: popIn 0.18s cubic-bezier(0.34, 1.3, 0.64, 1);
	}
	@keyframes popIn {
		from {
			opacity: 0;
			transform: translate(-50%, -50%) scale(0.96);
		}
		to {
			opacity: 1;
			transform: translate(-50%, -50%) scale(1);
		}
	}

	/* ── MODAL LEFT NAV ── */
	.modal-nav {
		width: 196px;
		min-width: 196px;
		background: var(--surface);
		display: flex;
		flex-direction: column;
		padding: 20px 10px 16px;
		overflow-y: auto;
	}

	.modal-nav-label {
		font-family: var(--mono);
		font-size: 10px;
		color: var(--text-3);
		letter-spacing: 0.1em;
		text-transform: uppercase;
		padding: 0 8px;
		margin-bottom: 10px;
	}

	.modal-nav-items {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.modal-nav-item {
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 8px 10px;
		/* Reset all button defaults that cause purple tints */
		border: none;
		outline: none;
		background: none;
		-webkit-appearance: none;
		appearance: none;
		color: var(--text-2);
		font-family: var(--sans);
		font-size: 13px;
		border-radius: 7px;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition:
			background 0.1s,
			color 0.1s;
		position: relative;
	}
	.modal-nav-item:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	.modal-nav-item:hover {
		background: var(--surface-2);
		color: var(--text);
	}
	.modal-nav-item.active {
		background: var(--surface-2);
		color: var(--text);
		font-weight: 500;
	}
	/* Accent left bar on active item */
	.modal-nav-item.active::before {
		content: "";
		position: absolute;
		left: 0;
		top: 20%;
		height: 60%;
		width: 2.5px;
		background: var(--accent);
		border-radius: 0 2px 2px 0;
	}

	.nav-item-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		color: var(--text-3);
		transition: color 0.1s;
	}
	.modal-nav-item:hover .nav-item-icon,
	.modal-nav-item.active .nav-item-icon {
		color: var(--accent);
	}

	.nav-item-label {
		font-size: 13px;
		line-height: 1;
	}

	/* ── DIVIDER ── */
	.modal-divider {
		width: 1px;
		background: var(--border);
		flex-shrink: 0;
	}

	/* ── MODAL CONTENT ──
	   Background is --bg so cards on --surface sit clearly above it. */
	.modal-content {
		flex: 1;
		position: relative;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--bg);
	}

	/* Sticky header with title + action buttons */
	.modal-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
		padding: 20px 28px 16px;
		flex-shrink: 0;
	}

	.modal-header-text {
		min-width: 0;
	}

	.modal-title {
		font-size: 17px;
		font-weight: 600;
		letter-spacing: -0.02em;
		color: var(--text);
		margin-bottom: 3px;
		line-height: 1.2;
	}

	.modal-subtitle {
		font-size: 13px;
		color: var(--text-2);
		line-height: 1.4;
	}

	.modal-header-actions {
		display: flex;
		align-items: center;
		gap: 2px;
		flex-shrink: 0;
		padding-top: 2px;
	}

	.header-action-btn {
		width: 28px;
		height: 28px;
		border: none;
		outline: none;
		background: none;
		-webkit-appearance: none;
		appearance: none;
		color: var(--text-3);
		border-radius: 6px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			background 0.12s,
			color 0.12s;
	}
	.header-action-btn:hover {
		background: var(--surface-2);
		color: var(--text-2);
	}
	.header-action-btn:focus-visible {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	/* Close gets a slightly stronger hover to signal it's destructive */
	.close-btn:hover {
		background: var(--surface-3);
		color: var(--text);
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	.spin {
		animation: spin 0.7s linear infinite;
	}

	.modal-view {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
		padding-bottom: 28px;
	}

	/* Child views render their own padding, no top padding needed since
	   the modal-header provides the breathing room */
	.modal-view :global(.node-stats),
	.modal-view :global(.sites),
	.modal-view :global(.publish) {
		padding-top: 4px;
	}
</style>
