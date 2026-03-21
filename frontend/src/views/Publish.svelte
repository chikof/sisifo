<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	let selectedPath = $state("");
	let siteName = $state("");
	let siteScope = $state("");
	let humanName = $state("");
	let publishing = $state(false);
	let publishedHash = $state("");
	let error = $state("");
	let copied = $state(false);
	let copiedPerm = $state(false);
	let permanentAddress = $state("");
	let publishedVersion = $state(0);
	let claimedName = $state<string | null>(null);

	// Auto-derive placeholder for human name input
	let humanNamePlaceholder = $derived(() => {
		const local = siteName
			.trim()
			.toLowerCase()
			.replace(/[^a-z0-9_-]/g, "-");
		const scope = siteScope.trim().toLowerCase();
		if (local && scope) return `${local}@${scope}`;
		if (local) return local;
		return "chiko@forum";
	});

	async function pickFolder() {
		const result = await invoke<string | null>("pick_folder");
		if (result) {
			selectedPath = result;
			if (!siteName) siteName = selectedPath.split("/").pop() ?? "";
		}
	}

	async function publish() {
		if (!selectedPath || !siteName.trim()) return;
		publishing = true;
		error = "";
		publishedHash = "";
		permanentAddress = "";
		claimedName = null;

		let nameToUse = humanName.trim() || null;
		if (!nameToUse && siteName.trim()) {
			const local = siteName
				.trim()
				.toLowerCase()
				.replace(/[^a-z0-9_-]/g, "-");
			nameToUse = siteScope.trim()
				? `${local}@${siteScope.trim()}`
				: local;
		}

		try {
			const result = await invoke<{
				hash: string;
				permanent_address: string;
				version: number;
				scope: string | null;
				claimed_name: string | null;
			}>("publish_site", {
				path: selectedPath,
				name: siteName.trim(),
				scope: siteScope.trim() ? siteScope.trim() : null,
				humanName: nameToUse,
			});
			publishedHash = result.hash;
			permanentAddress = result.permanent_address;
			publishedVersion = result.version;
			claimedName = result.claimed_name;
		} catch (e: any) {
			error = e?.toString() ?? "Publish failed";
		} finally {
			publishing = false;
		}
	}

	async function copy() {
		await navigator.clipboard.writeText(`sisi://${publishedHash}`);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	async function copyPerm() {
		await navigator.clipboard.writeText(`sisi://${permanentAddress}`);
		copiedPerm = true;
		setTimeout(() => (copiedPerm = false), 2000);
	}

	function reset() {
		selectedPath = "";
		siteName = "";
		siteScope = "";
		humanName = "";
		publishedHash = "";
		permanentAddress = "";
		claimedName = null;
		error = "";
	}
</script>

<div class="publish">
	<div class="header">
		<h1>Publish a site</h1>
		<p>
			Select a folder containing your site's files. It will be hashed,
			stored, and announced to the network.
		</p>
	</div>

	{#if publishedHash}
		<div class="success-card">
			<div class="success-icon">
				<svg
					width="22"
					height="22"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
			</div>
			<div class="success-body">
				<p class="success-title">Published — v{publishedVersion}</p>
				<p class="success-name">{siteName}</p>

				{#if claimedName}
					<div class="name-badge">
						<svg
							width="12"
							height="12"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"
							/>
							<line x1="7" y1="7" x2="7.01" y2="7" />
						</svg>
						<span>Name claimed: <strong>{claimedName}</strong></span
						>
					</div>
					<p class="name-hint">
						Peers subscribed to this topic will be able to reach you
						via
						<code>sisi://{claimedName}</code>.
					</p>
				{/if}

				<div class="addr-block">
					<div class="addr-label">
						Permanent address
						<span class="addr-hint"
							>share this — always points to the latest version</span
						>
					</div>
					<div class="hash-row">
						<code class="hash perm">{permanentAddress}</code>
						<button
							class="copy-btn"
							onclick={copyPerm}
							title="Copy permanent address"
						>
							{#if copiedPerm}
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.5"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<polyline points="20 6 9 17 4 12" />
								</svg>
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
								>
									<rect
										x="9"
										y="9"
										width="13"
										height="13"
										rx="2"
									/>
									<path
										d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
									/>
								</svg>
							{/if}
						</button>
					</div>
				</div>

				<div class="addr-block">
					<div class="addr-label">
						Version hash
						<span class="addr-hint">this specific version only</span
						>
					</div>
					<div class="hash-row">
						<code class="hash">{publishedHash}</code>
						<button
							class="copy-btn"
							onclick={copy}
							title="Copy version hash"
						>
							{#if copied}
								<svg
									width="13"
									height="13"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2.5"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<polyline points="20 6 9 17 4 12" />
								</svg>
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
								>
									<rect
										x="9"
										y="9"
										width="13"
										height="13"
										rx="2"
									/>
									<path
										d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
									/>
								</svg>
							{/if}
						</button>
					</div>
				</div>

				<p class="success-hint">
					Republish anytime from the same folder — the permanent
					address and name will automatically point to the new
					version.
				</p>
			</div>
			<button class="secondary-btn" onclick={reset}
				>Publish another</button
			>
		</div>
	{:else}
		<div class="form">
			<div class="field">
				<label for="folder">Site folder</label>
				<div class="folder-row">
					<div
						class={["folder-display", selectedPath && "selected"]}
						onclick={pickFolder}
						role="button"
						tabindex="0"
						onkeydown={(e) => e.key === "Enter" && pickFolder()}
					>
						{#if selectedPath}
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.8"
								stroke-linecap="round"
								stroke-linejoin="round"
								style="color: var(--accent); flex-shrink: 0"
							>
								<path
									d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
								/>
							</svg>
							<span class="folder-path">{selectedPath}</span>
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
								style="color: var(--text-3); flex-shrink: 0"
							>
								<path
									d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
								/>
							</svg>
							<span class="folder-placeholder"
								>Click to select a folder…</span
							>
						{/if}
					</div>
					<button class="browse-btn" onclick={pickFolder}
						>Browse</button
					>
				</div>
			</div>

			<div class="field">
				<label for="name">Site name</label>
				<input
					id="name"
					type="text"
					bind:value={siteName}
					placeholder="my-site"
					maxlength={64}
					spellcheck={false}
				/>
				<span class="field-hint"
					>Human-readable label stored in the manifest.</span
				>
			</div>

			<div class="field">
				<label for="scope">
					Topic scope <span class="optional-badge">optional</span>
				</label>
				<input
					id="scope"
					type="text"
					bind:value={siteScope}
					placeholder="e.g. forum, blog, portfolio"
					maxlength={32}
					spellcheck={false}
					autocomplete="off"
				/>
				<span class="field-hint">
					Group your site under a gossip topic. Peers in that topic
					will discover it automatically.
				</span>
			</div>

			<div class="field">
				<label for="human-name">
					Human-readable name <span class="optional-badge"
						>optional</span
					>
				</label>
				<input
					id="human-name"
					type="text"
					bind:value={humanName}
					placeholder={humanNamePlaceholder()}
					maxlength={64}
					spellcheck={false}
					autocomplete="off"
				/>
				<span class="field-hint">
					Leave blank to auto-derive from site name + scope. Format:
					<code>local</code> or <code>local@scope</code> — only
					<code>[a-z0-9_-]</code> characters.
				</span>
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
					>
						<circle cx="12" cy="12" r="10" />
						<line x1="12" y1="8" x2="12" y2="12" />
						<line x1="12" y1="16" x2="12.01" y2="16" />
					</svg>
					{error}
				</div>
			{/if}

			<button
				class="primary-btn"
				onclick={publish}
				disabled={!selectedPath || !siteName.trim() || publishing}
			>
				{#if publishing}
					<span class="btn-spinner"></span>
					Publishing…
				{:else}
					<svg
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
						<polyline points="17 8 12 3 7 8" />
						<line x1="12" y1="3" x2="12" y2="15" />
					</svg>
					Publish to network
				{/if}
			</button>

			<p class="disclaimer">
				Files are hashed locally, then announced via the iroh DHT.
				Nothing is uploaded to a central server.
			</p>
		</div>
	{/if}
</div>

<style>
	.publish {
		padding: 32px 36px;
		max-width: 560px;
		display: flex;
		flex-direction: column;
		gap: 28px;
	}

	.header h1 {
		font-size: 18px;
		font-weight: 600;
		letter-spacing: -0.02em;
		color: var(--text);
		margin-bottom: 6px;
	}
	.header p {
		font-size: 13.5px;
		color: var(--text-2);
		line-height: 1.55;
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	label {
		font-size: 12.5px;
		font-weight: 500;
		color: var(--text-2);
		letter-spacing: 0.01em;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.optional-badge {
		font-size: 10px;
		font-weight: 400;
		color: var(--text-3);
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 0 5px;
		letter-spacing: 0.03em;
	}

	.folder-row {
		display: flex;
		gap: 8px;
	}

	.folder-display {
		flex: 1;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-2);
		cursor: pointer;
		min-width: 0;
		transition: border-color 0.15s;
	}
	.folder-display:hover {
		border-color: var(--accent);
	}
	.folder-display.selected {
		border-color: var(--accent);
		background: var(--accent-light);
	}

	.folder-path {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.folder-placeholder {
		font-size: 13px;
		color: var(--text-3);
	}

	.browse-btn {
		padding: 0 14px;
		height: 36px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-2);
		color: var(--text-2);
		font-family: "DM Sans", sans-serif;
		font-size: 13px;
		cursor: pointer;
		transition:
			border-color 0.15s,
			color 0.15s;
		white-space: nowrap;
		flex-shrink: 0;
	}
	.browse-btn:hover {
		border-color: var(--accent);
		color: var(--accent);
	}

	input[type="text"] {
		padding: 8px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface-2);
		font-family: "DM Sans", sans-serif;
		font-size: 13.5px;
		color: var(--text);
		outline: none;
		transition:
			border-color 0.15s,
			box-shadow 0.15s;
	}
	input[type="text"]:focus {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 15%, transparent);
	}
	input::placeholder {
		color: var(--text-3);
	}

	.field-hint {
		font-size: 12px;
		color: var(--text-3);
		line-height: 1.5;
	}
	.field-hint code {
		font-family: var(--mono);
		font-size: 11px;
		color: var(--accent);
		background: var(--accent-light);
		padding: 1px 4px;
		border-radius: 3px;
	}

	.primary-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 10px 20px;
		background: var(--accent);
		color: white;
		border: none;
		border-radius: var(--radius);
		font-family: "DM Sans", sans-serif;
		font-size: 13.5px;
		font-weight: 500;
		cursor: pointer;
		transition:
			opacity 0.15s,
			transform 0.1s;
	}
	.primary-btn:hover:not(:disabled) {
		opacity: 0.9;
	}
	.primary-btn:active:not(:disabled) {
		transform: scale(0.99);
	}
	.primary-btn:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.btn-spinner {
		width: 13px;
		height: 13px;
		border: 2px solid rgba(255, 255, 255, 0.3);
		border-top-color: white;
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.disclaimer {
		font-size: 12px;
		color: var(--text-3);
		line-height: 1.5;
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

	/* ── Success state ── */
	.success-card {
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}
	.success-icon {
		width: 36px;
		height: 36px;
		background: var(--accent-light);
		color: var(--accent);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.success-body {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.success-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text);
	}
	.success-name {
		font-size: 13px;
		color: var(--text-2);
	}

	.name-badge {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 8px 12px;
		background: var(--accent-light);
		border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
		border-radius: var(--radius);
		font-size: 13px;
		color: var(--accent);
		margin-top: 8px;
	}
	.name-badge strong {
		font-weight: 600;
	}
	.name-hint {
		font-size: 12px;
		color: var(--text-3);
		line-height: 1.5;
	}
	.name-hint code {
		font-family: var(--mono);
		font-size: 11px;
		color: var(--accent);
		background: var(--accent-light);
		padding: 1px 4px;
		border-radius: 3px;
	}

	.hash-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-top: 4px;
	}
	.hash {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
		background: var(--accent-light);
		padding: 4px 8px;
		border-radius: 4px;
		word-break: break-all;
		flex: 1;
	}
	.hash.perm {
		border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
	}
	.copy-btn {
		width: 28px;
		height: 28px;
		border: 1px solid var(--border);
		border-radius: 4px;
		background: var(--surface-3);
		color: var(--text-2);
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

	.addr-block {
		display: flex;
		flex-direction: column;
		gap: 5px;
		margin-top: 10px;
	}
	.addr-label {
		font-size: 12px;
		font-weight: 500;
		color: var(--text-2);
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.addr-hint {
		font-size: 11px;
		color: var(--text-3);
		font-weight: 400;
	}

	.success-hint {
		font-size: 12px;
		color: var(--text-3);
		line-height: 1.5;
		margin-top: 4px;
	}
	.secondary-btn {
		padding: 8px 16px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: none;
		color: var(--text-2);
		font-family: "DM Sans", sans-serif;
		font-size: 13px;
		cursor: pointer;
		align-self: flex-start;
		transition:
			border-color 0.15s,
			color 0.15s;
	}
	.secondary-btn:hover {
		border-color: var(--accent);
		color: var(--accent);
	}
</style>
