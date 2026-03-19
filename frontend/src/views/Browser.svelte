<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";

	let address = "";
	let inputValue = "";
	let loading = false;
	let error = "";
	let gatewayUrl = "";
	let history: string[] = [];
	let historyIndex = -1;

	function sanitizeInput(value: string) {
		return value.replace(/^sisi:\/\//, "");
	}

	function onInput(e: Event) {
		const target = e.target as HTMLInputElement;
		inputValue = sanitizeInput(target.value);
	}

	async function navigate(addr?: string) {
		const target = sanitizeInput(addr ?? inputValue.trim());
		if (!target) return;

		loading = true;
		error = "";

		try {
			const url = await invoke<string>("resolve_address", {
				addr: target,
			});
			gatewayUrl = url;
			address = target;
			inputValue = target;

			// Push to history
			history = [...history.slice(0, historyIndex + 1), target];
			historyIndex = history.length - 1;
		} catch (e: any) {
			error = e?.toString() ?? "Failed to resolve address";
			gatewayUrl = "";
		} finally {
			loading = false;
		}
	}

	function goBack() {
		if (historyIndex > 0) {
			historyIndex--;
			navigate(history[historyIndex]);
		}
	}

	function goForward() {
		if (historyIndex < history.length - 1) {
			historyIndex++;
			navigate(history[historyIndex]);
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Enter") navigate();
	}

	function onFocus(e: FocusEvent) {
		(e.target as HTMLInputElement).select();
	}

	// Example addresses to demo
	const examples = ["sisi://bafyreiabc123…", "sisi://bafyreidef456…"];
</script>

<div class="browser">
	<div class="toolbar">
		<div class="nav-buttons">
			<button
				class="icon-btn"
				on:click={goBack}
				disabled={historyIndex <= 0}
				title="Back"
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
					><polyline points="15 18 9 12 15 6" /></svg
				>
			</button>
			<button
				class="icon-btn"
				on:click={goForward}
				disabled={historyIndex >= history.length - 1}
				title="Forward"
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
					><polyline points="9 18 15 12 9 6" /></svg
				>
			</button>
		</div>

		<div class="address-bar" class:loading>
			<span class="scheme">sisi://</span>
			<input
				type="text"
				bind:value={inputValue}
				on:input={onInput}
				on:keydown={onKeydown}
				on:focus={onFocus}
				placeholder="enter site hash or sisi:// address"
				spellcheck="false"
				autocomplete="off"
			/>
			{#if loading}
				<span class="spinner"></span>
			{:else}
				<button
					class="go-btn"
					aria-label="ye"
					on:click={() => navigate()}
					disabled={!inputValue.trim()}
				>
					<svg
						width="13"
						height="13"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.2"
						stroke-linecap="round"
						stroke-linejoin="round"
						><polyline points="9 18 15 12 9 6" /></svg
					>
				</button>
			{/if}
		</div>
	</div>

	<div class="webview-area">
		{#if error}
			<div class="error-state">
				<svg
					width="32"
					height="32"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					stroke-linecap="round"
					stroke-linejoin="round"
					style="color: #d97706"
					><circle cx="12" cy="12" r="10" /><line
						x1="12"
						y1="8"
						x2="12"
						y2="12"
					/><line x1="12" y1="16" x2="12.01" y2="16" /></svg
				>
				<p class="error-msg">{error}</p>
				<p class="error-hint">
					Make sure the address is a valid site hash and that at least
					one peer is seeding it.
				</p>
			</div>
		{:else if !gatewayUrl}
			<div class="empty-state">
				<div class="empty-icon">
					<svg
						width="40"
						height="40"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.2"
						stroke-linecap="round"
						stroke-linejoin="round"
						style="color: var(--text-3)"
					>
						<circle cx="12" cy="12" r="10" /><line
							x1="2"
							y1="12"
							x2="22"
							y2="12"
						/>
						<path
							d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"
						/>
					</svg>
				</div>
				<p class="empty-title">Enter a site address to browse</p>
				<p class="empty-sub">
					Paste a <code>sisi://</code> address or a raw content hash above
				</p>
			</div>
		{:else}
			<!-- In Tauri, this would be a <webview> tag. We use iframe here for preview. -->
			<iframe
				src={gatewayUrl}
				title="Sisifo site"
				sandbox="allow-scripts allow-same-origin"
			></iframe>
		{/if}
	</div>
</div>

<style>
	.browser {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-2);
	}

	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		background: var(--surface);
		border-bottom: 1px solid var(--border);
	}

	.nav-buttons {
		display: flex;
		gap: 2px;
		flex-shrink: 0;
	}

	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		background: none;
		color: var(--text-2);
		border-radius: 4px;
		cursor: pointer;
		transition:
			background 0.1s,
			color 0.1s;
	}
	.icon-btn:hover:not(:disabled) {
		background: var(--surface-2);
		color: var(--text);
	}
	.icon-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.address-bar {
		flex: 1;
		display: flex;
		align-items: center;
		background: var(--surface-2);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0 4px 0 10px;
		height: 34px;
		gap: 4px;
		transition:
			border-color 0.15s,
			box-shadow 0.15s;
	}
	.address-bar:focus-within {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 15%, transparent);
		background: var(--surface);
	}
	.address-bar.loading {
		opacity: 0.7;
	}

	.scheme {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
		flex-shrink: 0;
		white-space: nowrap;
	}

	.address-bar input {
		flex: 1;
		border: none;
		background: none;
		font-family: var(--mono);
		font-size: 12.5px;
		color: var(--text);
		outline: none;
		min-width: 0;
	}
	.address-bar input::placeholder {
		color: var(--text-3);
		font-family: "DM Sans", sans-serif;
	}

	.go-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 4px;
		cursor: pointer;
		transition:
			color 0.1s,
			background 0.1s;
		flex-shrink: 0;
	}
	.go-btn:hover:not(:disabled) {
		color: var(--accent);
		background: var(--accent-light);
	}
	.go-btn:disabled {
		opacity: 0.3;
		cursor: default;
	}

	.spinner {
		width: 14px;
		height: 14px;
		border: 2px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		flex-shrink: 0;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.webview-area {
		flex: 1;
		position: relative;
		overflow: hidden;
	}

	iframe {
		width: 100%;
		height: 100%;
		border: none;
		background: var(--surface);
	}

	.empty-state,
	.error-state {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 40px;
		text-align: center;
	}

	.empty-icon {
		margin-bottom: 4px;
		opacity: 0.5;
	}
	.empty-title {
		font-size: 15px;
		font-weight: 500;
		color: var(--text-2);
	}
	.empty-sub {
		font-size: 13px;
		color: var(--text-3);
		max-width: 340px;
		line-height: 1.5;
	}
	.empty-sub code {
		font-family: var(--mono);
		font-size: 12px;
		color: var(--accent);
		background: var(--accent-light);
		padding: 1px 5px;
		border-radius: 3px;
	}

	.error-msg {
		font-size: 14px;
		font-weight: 500;
		color: #92400e;
	}
	.error-hint {
		font-size: 13px;
		color: var(--text-3);
		max-width: 340px;
		line-height: 1.5;
	}
</style>
