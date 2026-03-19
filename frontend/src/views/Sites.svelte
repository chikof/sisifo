<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";

	interface SiteMeta {
		name: string;
		hash: string;
		file_count: number;
		total_size: number;
		updated_at: number;
	}

	let sites: SiteMeta[] = [];
	let loading = true;
	let error = "";
	let unpinning: string | null = null;
	let copied: string | null = null;

	onMount(load);

	async function load() {
		loading = true;
		error = "";
		try {
			sites = await invoke<SiteMeta[]>("list_local_sites");
		} catch (e: any) {
			error = e?.toString() ?? "Failed to load sites";
		} finally {
			loading = false;
		}
	}

	async function unpin(hash: string) {
		unpinning = hash;
		try {
			await invoke("unpin_site", { hash });
			sites = sites.filter((s) => s.hash !== hash);
		} catch (e: any) {
			error = e?.toString() ?? "Unpin failed";
		} finally {
			unpinning = null;
		}
	}

	async function copy(hash: string) {
		await navigator.clipboard.writeText(`sisi://${hash}`);
		copied = hash;
		setTimeout(() => (copied = null), 2000);
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function formatDate(ts: number): string {
		return new Date(ts * 1000).toLocaleDateString(undefined, {
			month: "short",
			day: "numeric",
			year: "numeric",
		});
	}
</script>

<div class="sites">
	<div class="header">
		<div>
			<h1>Hosted sites</h1>
			<p>Sites you're seeding on the network.</p>
		</div>
		<button
			class="refresh-btn"
			on:click={load}
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

	{#if loading}
		<div class="loading-state">
			<span class="spinner"></span>
			<span>Loading sites…</span>
		</div>
	{:else if sites.length === 0}
		<div class="empty-state">
			<svg
				width="36"
				height="36"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="1.2"
				stroke-linecap="round"
				stroke-linejoin="round"
				style="color: var(--text-3)"
				><rect x="2" y="3" width="20" height="14" rx="2" /><line
					x1="8"
					y1="21"
					x2="16"
					y2="21"
				/><line x1="12" y1="17" x2="12" y2="21" /></svg
			>
			<p class="empty-title">No sites hosted yet</p>
			<p class="empty-sub">Publish a site and it will appear here.</p>
		</div>
	{:else}
		<div class="site-list">
			{#each sites as site (site.hash)}
				<div class="site-card">
					<div class="site-main">
						<div class="site-name">{site.name}</div>
						<div class="site-meta">
							<span
								>{site.file_count} file{site.file_count !== 1
									? "s"
									: ""}</span
							>
							<span class="sep">·</span>
							<span>{formatSize(site.total_size)}</span>
							<span class="sep">·</span>
							<span>{formatDate(site.updated_at)}</span>
						</div>
						<div class="hash-row">
							<code class="hash">{site.hash.slice(0, 20)}…</code>
							<button
								class="copy-btn"
								on:click={() => copy(site.hash)}
								title="Copy full address"
							>
								{#if copied === site.hash}
									<svg
										width="12"
										height="12"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										stroke-linecap="round"
										stroke-linejoin="round"
										><polyline
											points="20 6 9 17 4 12"
										/></svg
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
						</div>
					</div>

					<div class="site-actions">
						<span class="seeding-badge">
							<span class="seed-dot"></span>
							seeding
						</span>
						<button
							class="unpin-btn"
							on:click={() => unpin(site.hash)}
							disabled={unpinning === site.hash}
							title="Stop seeding this site"
						>
							{#if unpinning === site.hash}
								<span class="btn-spinner"></span>
							{:else}
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
									><polyline points="3 6 5 6 21 6" /><path
										d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"
									/><path d="M10 11v6" /><path
										d="M14 11v6"
									/><path
										d="M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
									/></svg
								>
							{/if}
							Unpin
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.sites {
		padding: 32px 36px;
		display: flex;
		flex-direction: column;
		gap: 24px;
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

	.loading-state {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-3);
		font-size: 13.5px;
		padding: 20px 0;
	}
	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		padding: 60px 20px;
		text-align: center;
	}
	.empty-title {
		font-size: 15px;
		font-weight: 500;
		color: var(--text-2);
	}
	.empty-sub {
		font-size: 13px;
		color: var(--text-3);
	}

	.site-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.site-card {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 16px 18px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		transition: border-color 0.15s;
	}
	.site-card:hover {
		border-color: #c8c4be;
	}

	.site-main {
		display: flex;
		flex-direction: column;
		gap: 5px;
		min-width: 0;
	}

	.site-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.site-meta {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 12px;
		color: var(--text-3);
	}
	.sep {
		color: var(--border);
	}

	.hash-row {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 2px;
	}
	.hash {
		font-family: var(--mono);
		font-size: 11.5px;
		color: var(--accent);
		background: var(--accent-light);
		padding: 2px 6px;
		border-radius: 3px;
	}
	.copy-btn {
		width: 22px;
		height: 22px;
		border: 1px solid var(--border);
		border-radius: 3px;
		background: var(--surface);
		color: var(--text-3);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition:
			border-color 0.12s,
			color 0.12s;
		flex-shrink: 0;
	}
	.copy-btn:hover {
		border-color: var(--accent);
		color: var(--accent);
	}

	.site-actions {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 8px;
		flex-shrink: 0;
	}

	.seeding-badge {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11.5px;
		color: var(--accent);
		font-weight: 500;
	}
	.seed-dot {
		width: 6px;
		height: 6px;
		background: var(--accent);
		border-radius: 50%;
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

	.unpin-btn {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 5px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: none;
		color: var(--text-3);
		font-family: "DM Sans", sans-serif;
		font-size: 12px;
		cursor: pointer;
		transition:
			border-color 0.15s,
			color 0.15s,
			background 0.15s;
	}
	.unpin-btn:hover:not(:disabled) {
		border-color: #f87171;
		color: #ef4444;
		background: #fef2f2;
	}
	.unpin-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.btn-spinner {
		width: 11px;
		height: 11px;
		border: 1.5px solid var(--border);
		border-top-color: var(--text-3);
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
