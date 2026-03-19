<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount, onDestroy } from "svelte";

	interface NodeStats {
		node_id: string;
		peer_count: number;
		bytes_sent: number;
		bytes_recv: number;
		hosted_sites: number;
		relay_url: string | null;
		is_online: boolean;
	}

	let stats: NodeStats | null = null;
	let error = "";
	let loading = true;
	let daemonRunning = false;
	let nodeIdentity = "";
	let copiedId = false;

	let interval: ReturnType<typeof setInterval>;

	onMount(async () => {
		await loadAll();
		interval = setInterval(loadStats, 5000);
	});

	onDestroy(() => clearInterval(interval));

	async function loadAll() {
		loading = true;
		try {
			[daemonRunning, nodeIdentity] = await Promise.all([
				invoke<boolean>("daemon_running"),
				invoke<string>("node_identity"),
			]);
			await loadStats();
		} catch (e: any) {
			error = e?.toString() ?? "Failed to load node info";
		} finally {
			loading = false;
		}
	}

	async function loadStats() {
		try {
			stats = await invoke<NodeStats>("node_stats");
			error = "";
		} catch (e: any) {
			error = e?.toString() ?? "Failed to fetch stats";
		}
	}

	async function copyId() {
		await navigator.clipboard.writeText(nodeIdentity);
		copiedId = true;
		setTimeout(() => (copiedId = false), 2000);
	}

	function formatBytes(b: number): string {
		if (b < 1024) return `${b} B`;
		if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
		return `${(b / (1024 * 1024)).toFixed(2)} MB`;
	}
</script>

<div class="node-stats">
	<div class="header">
		<div>
			<h1>Node</h1>
			<p>Live stats for your local iroh node.</p>
		</div>
		<button
			class="refresh-btn"
			on:click={loadAll}
			disabled={loading}
			title="Refresh"
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
				class:spin={loading}
				><polyline points="23 4 23 10 17 10" /><polyline
					points="1 20 1 14 7 14"
				/><path
					d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"
				/></svg
			>
		</button>
	</div>

	{#if error}
		<div class="error-banner">
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				><circle cx="12" cy="12" r="10" /><line
					x1="12"
					y1="8"
					x2="12"
					y2="12"
				/><line x1="12" y1="16" x2="12.01" y2="16" /></svg
			>
			{error}
		</div>
	{/if}

	<div class="section">
		<div class="section-title">Identity</div>
		<div class="identity-card">
			<div class="id-label">Node ID</div>
			<div class="id-row">
				<code class="id-value">{nodeIdentity || "—"}</code>
				{#if nodeIdentity}
					<button
						class="copy-btn"
						on:click={copyId}
						title="Copy node ID"
					>
						{#if copiedId}
							<svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2.5"
								stroke-linecap="round"
								stroke-linejoin="round"
								><polyline points="20 6 9 17 4 12" /></svg
							>
						{:else}
							<svg
								width="12"
								height="12"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
								><rect
									x="9"
									y="9"
									width="13"
									height="13"
									rx="2"
								/><path
									d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
								/></svg
							>
						{/if}
					</button>
				{/if}
			</div>
			<p class="id-hint">
				This is your persistent public identity on the network. Derived
				from your ed25519 keypair.
			</p>
		</div>
	</div>

	<div class="section">
		<div class="section-title">Daemon</div>
		<div class="daemon-card" class:online={daemonRunning}>
			<div class="daemon-status">
				<span class="status-dot" class:online={daemonRunning}></span>
				<span class="status-label"
					>{daemonRunning
						? "sisid is running"
						: "sisid is offline"}</span
				>
			</div>
			<p class="daemon-desc">
				{#if daemonRunning}
					Sites are being seeded persistently in the background, even
					when this app is closed.
				{:else}
					The daemon is not running. Sites will only be seeded while
					this app is open. Enable sisid via systemd to seed
					persistently.
				{/if}
			</p>
			{#if !daemonRunning}
				<code class="daemon-cmd">systemctl --user start sisid</code>
			{/if}
		</div>
	</div>

	{#if stats}
		<div class="section">
			<div class="section-title">Statistics</div>
			<div class="stats-grid">
				<div class="stat-card">
					<div class="stat-value">{stats.peer_count}</div>
					<div class="stat-label">Connected peers</div>
				</div>
				<div class="stat-card">
					<div class="stat-value">{stats.hosted_sites}</div>
					<div class="stat-label">Seeding</div>
				</div>
				<div class="stat-card">
					<div class="stat-value">
						{formatBytes(stats.bytes_sent)}
					</div>
					<div class="stat-label">Uploaded</div>
				</div>
				<div class="stat-card">
					<div class="stat-value">
						{formatBytes(stats.bytes_recv)}
					</div>
					<div class="stat-label">Downloaded</div>
				</div>
			</div>
			{#if stats.relay_url}
				<div class="section">
					<div class="section-title">Relay</div>
					<div class="identity-card">
						<div class="id-label">Home relay</div>
						<code class="id-value">{stats.relay_url}</code>
					</div>
				</div>
			{/if}
		</div>
	{:else if loading}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>Loading node stats…</span>
		</div>
	{/if}
</div>

<style>
	.node-stats {
		padding: 32px 36px;
		display: flex;
		flex-direction: column;
		gap: 28px;
		max-width: 600px;
		height: 100%;
		overflow-y: auto;
	}

	.header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}
	.header h1 {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.02em;
		color: var(--text);
		margin-bottom: 4px;
	}
	.header p {
		font-size: 13.5px;
		color: var(--text-2);
	}

	.refresh-btn {
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
		color: var(--text-2);
		cursor: pointer;
		transition:
			border-color 0.15s,
			color 0.15s;
		flex-shrink: 0;
	}
	.refresh-btn:hover:not(:disabled) {
		border-color: var(--accent);
		color: var(--accent);
	}

	.spin {
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}
	.section-title {
		font-size: 11.5px;
		font-weight: 600;
		color: var(--text-3);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.identity-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 16px 18px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.id-label {
		font-size: 12px;
		font-weight: 500;
		color: var(--text-2);
	}
	.id-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.id-value {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--text);
		word-break: break-all;
		flex: 1;
		line-height: 1.5;
	}
	.copy-btn {
		width: 26px;
		height: 26px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface);
		color: var(--text-3);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		transition:
			border-color 0.12s,
			color 0.12s;
	}
	.copy-btn:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
	.id-hint {
		font-size: 12px;
		color: var(--text-3);
		line-height: 1.5;
	}

	.daemon-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 16px 18px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.daemon-card.online {
		border-color: color-mix(in srgb, var(--accent) 30%, var(--border));
	}

	.daemon-status {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--text-3);
		flex-shrink: 0;
	}
	.status-dot.online {
		background: var(--accent);
		animation: pulse 2s ease-in-out infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
	}
	.status-label {
		font-size: 13.5px;
		font-weight: 500;
		color: var(--text);
	}
	.daemon-desc {
		font-size: 13px;
		color: var(--text-2);
		line-height: 1.55;
	}
	.daemon-cmd {
		display: block;
		font-family: var(--mono);
		font-size: 12px;
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 4px;
		padding: 8px 12px;
		color: var(--text);
		margin-top: 4px;
		user-select: all;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.stat-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 16px 18px;
	}
	.stat-value {
		font-size: 22px;
		font-weight: 600;
		letter-spacing: -0.03em;
		color: var(--text);
		font-variant-numeric: tabular-nums;
		margin-bottom: 4px;
	}
	.stat-label {
		font-size: 12px;
		color: var(--text-3);
	}

	.loading-state {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-3);
		font-size: 13.5px;
	}
	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	.error-banner {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		background: #fef3c7;
		border: 1px solid #fde68a;
		border-radius: var(--radius);
		font-size: 13px;
		color: #92400e;
	}
</style>
