<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onMount, onDestroy } from "svelte";

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

	export let tabs: Tab[];
	export let activeTabId: string;
	export let groups: TabGroup[];

	// Drag state
	let dragTabId: string | null = null;
	let dragOverTabId: string | null = null;

	// Context menu
	let ctxMenu: { x: number; y: number; tabId: string } | null = null;

	// Group creation modal
	let showGroupModal = false;
	let groupModalTabId: string | null = null;
	let newGroupName = "";
	let newGroupColor = "#0d9488";

	// Gossip bridge
	let gossipUnlisten: (() => void) | null = null;
	let activeIframe: HTMLIFrameElement | null = null;

	const GROUP_COLORS = [
		"#0d9488",
		"#0369a1",
		"#7c3aed",
		"#be185d",
		"#c2410c",
		"#15803d",
		"#b45309",
		"#4338ca",
	];

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

	$: activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0];

	function openTab(address = "", groupId: string | null = null) {
		const tab = newTab(address, groupId);
		tabs = [...tabs, tab];
		activeTabId = tab.id;
		if (address) navigateTab(tab.id, address);
	}

	function closeTab(id: string) {
		if (tabs.length === 1) {
			// Reset to blank instead of closing last tab
			tabs = [newTab()];
			activeTabId = tabs[0].id;
			return;
		}
		const idx = tabs.findIndex((t) => t.id === id);
		tabs = tabs.filter((t) => t.id !== id);
		if (activeTabId === id) {
			activeTabId = tabs[Math.max(0, idx - 1)].id;
		}
	}

	function duplicateTab(id: string) {
		const src = tabs.find((t) => t.id === id);
		if (!src) return;
		const tab = newTab(src.address, src.groupId);
		tab.gatewayUrl = src.gatewayUrl;
		tab.title = src.title;
		const idx = tabs.findIndex((t) => t.id === id);
		tabs = [...tabs.slice(0, idx + 1), tab, ...tabs.slice(idx + 1)];
		activeTabId = tab.id;
	}

	function updateTab(id: string, patch: Partial<Tab>) {
		tabs = tabs.map((t) => (t.id === id ? { ...t, ...patch } : t));
	}

	function sanitize(v: string) {
		return v.replace(/^sisi:\/\//, "").trim();
	}

	async function navigateTab(id: string, addr?: string) {
		const tab = tabs.find((t) => t.id === id);
		if (!tab) return;
		const target = sanitize(addr ?? tab.inputValue);
		if (!target) return;

		updateTab(id, { loading: true, error: "" });

		try {
			const url = await invoke<string>("resolve_address", {
				addr: target,
			});
			const title = target.slice(0, 20) + (target.length > 20 ? "…" : "");
			const history = [
				...tab.history.slice(0, tab.historyIndex + 1),
				target,
			];
			updateTab(id, {
				gatewayUrl: url,
				address: target,
				inputValue: target,
				title,
				loading: false,
				history,
				historyIndex: history.length - 1,
			});
		} catch (e: any) {
			updateTab(id, {
				error: e?.toString() ?? "Failed",
				loading: false,
				gatewayUrl: "",
			});
		}
	}

	function goBack(id: string) {
		const tab = tabs.find((t) => t.id === id);
		if (!tab || tab.historyIndex <= 0) return;
		navigateTab(id, tab.history[tab.historyIndex - 1]);
	}

	function goForward(id: string) {
		const tab = tabs.find((t) => t.id === id);
		if (!tab || tab.historyIndex >= tab.history.length - 1) return;
		navigateTab(id, tab.history[tab.historyIndex + 1]);
	}

	function onDragStart(e: DragEvent, id: string) {
		dragTabId = id;
		e.dataTransfer!.effectAllowed = "move";
	}

	function onDragOver(e: DragEvent, id: string) {
		e.preventDefault();
		dragOverTabId = id;
	}

	function onDrop(e: DragEvent, targetId: string) {
		e.preventDefault();
		if (!dragTabId || dragTabId === targetId) {
			dragTabId = null;
			dragOverTabId = null;
			return;
		}
		const from = tabs.findIndex((t) => t.id === dragTabId);
		const to = tabs.findIndex((t) => t.id === targetId);
		const reordered = [...tabs];
		const [moved] = reordered.splice(from, 1);
		reordered.splice(to, 0, moved);
		tabs = reordered;
		dragTabId = null;
		dragOverTabId = null;
	}

	function onDragEnd() {
		dragTabId = null;
		dragOverTabId = null;
	}

	function showCtxMenu(e: MouseEvent, tabId: string) {
		e.preventDefault();
		ctxMenu = { x: e.clientX, y: e.clientY, tabId };
	}

	function closeCtxMenu() {
		ctxMenu = null;
	}

	function ctxAction(action: string) {
		if (!ctxMenu) return;
		const id = ctxMenu.tabId;
		closeCtxMenu();
		switch (action) {
			case "close":
				closeTab(id);
				break;
			case "duplicate":
				duplicateTab(id);
				break;
			case "new-tab":
				openTab();
				break;
			case "new-group":
				groupModalTabId = id;
				showGroupModal = true;
				break;
			case "remove-group":
				updateTab(id, { groupId: null });
				break;
		}
	}

	function createGroup() {
		if (!newGroupName.trim()) return;
		const group: TabGroup = {
			id: uid(),
			name: newGroupName.trim(),
			color: newGroupColor,
			collapsed: false,
		};
		groups = [...groups, group];
		if (groupModalTabId) updateTab(groupModalTabId, { groupId: group.id });
		showGroupModal = false;
		newGroupName = "";
		groupModalTabId = null;
	}

	function toggleGroupCollapse(groupId: string) {
		groups = groups.map((g) =>
			g.id === groupId ? { ...g, collapsed: !g.collapsed } : g,
		);
		// If active tab is in collapsed group, switch to first visible tab
		const activeGroup = groups.find((g) => g.id === groupId);
		if (activeGroup?.collapsed) {
			const tab = tabs.find((t) => t.id === activeTabId);
			if (tab?.groupId === groupId) {
				const visible = tabs.find(
					(t) =>
						!t.groupId ||
						!groups.find((g) => g.id === t.groupId)?.collapsed,
				);
				if (visible) activeTabId = visible.id;
			}
		}
	}

	function groupColor(groupId: string | null) {
		if (!groupId) return null;
		return groups.find((g) => g.id === groupId)?.color ?? null;
	}

	function groupName(groupId: string | null) {
		if (!groupId) return null;
		return groups.find((g) => g.id === groupId)?.name ?? null;
	}

	// Tabs ordered: group tabs cluster together, ungrouped tabs in between
	$: orderedTabs = (() => {
		const result: (Tab | TabGroup)[] = [];
		const seen = new Set<string>();
		for (const tab of tabs) {
			if (tab.groupId && !seen.has(tab.groupId)) {
				const group = groups.find((g) => g.id === tab.groupId);
				if (group) {
					result.push(group);
					seen.add(tab.groupId);
				}
			}
			const group = tab.groupId
				? groups.find((g) => g.id === tab.groupId)
				: null;
			if (!group?.collapsed) result.push(tab);
		}
		return result;
	})();

	async function handleBridgeMessage(event: MessageEvent) {
		if (
			!event.origin.startsWith("http://localhost:7777") &&
			!event.origin.startsWith("http://127.0.0.1:7777")
		)
			return;
		const { id, cmd, args } = event.data ?? {};
		if (!id || !cmd) return;

		const reply = (result?: any, err?: string) => {
			(event.source as WindowProxy)?.postMessage(
				err ? { id, error: err } : { id, result },
				{ targetOrigin: event.origin },
			);
		};

		if (cmd === "subscribe_topic") {
			gossipUnlisten?.();
			try {
				gossipUnlisten = await listen(`gossip:${args?.topic}`, (e) => {
					activeIframe?.contentWindow?.postMessage(
						{
							tauriEvent: `gossip:${args?.topic}`,
							payload: e.payload,
						},
						event.origin,
					);
				});
				reply(null);
			} catch (e: any) {
				reply(undefined, String(e));
			}
			return;
		}

		try {
			reply(await invoke(cmd, args ?? {}));
		} catch (e: any) {
			reply(undefined, String(e));
		}
	}

	onMount(() => {
		window.addEventListener("message", handleBridgeMessage);
	});
	onDestroy(() => {
		window.removeEventListener("message", handleBridgeMessage);
		gossipUnlisten?.();
	});

	// Keyboard shortcuts
	function onWindowKeydown(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey) {
			if (e.key === "t") {
				e.preventDefault();
				openTab();
			}
			if (e.key === "w") {
				e.preventDefault();
				closeTab(activeTabId);
			}
			if (e.key === "Tab") {
				e.preventDefault();
				const visibleTabs = tabs.filter((t) => {
					const g = t.groupId
						? groups.find((g) => g.id === t.groupId)
						: null;
					return !g?.collapsed;
				});
				const idx = visibleTabs.findIndex((t) => t.id === activeTabId);
				const next = e.shiftKey
					? visibleTabs[
							(idx - 1 + visibleTabs.length) % visibleTabs.length
						]
					: visibleTabs[(idx + 1) % visibleTabs.length];
				if (next) activeTabId = next.id;
			}
		}
	}
</script>

<svelte:window on:keydown={onWindowKeydown} on:click={closeCtxMenu} />

<div class="browser">
	<!-- TAB BAR -->
	<div class="tab-bar">
		{#each orderedTabs as item ("id" in item && "address" in item ? item.id : `group-${item.id}`)}
			{#if "address" in item}
				<!-- Tab -->
				{@const tab = item}
				{@const color = groupColor(tab.groupId)}
				<div
					class="tab"
					class:active={tab.id === activeTabId}
					class:drag-over={dragOverTabId === tab.id}
					style={color ? `--tab-group-color: ${color}` : ""}
					draggable="true"
					on:click={() => (activeTabId = tab.id)}
					on:contextmenu={(e) => showCtxMenu(e, tab.id)}
					on:dragstart={(e) => onDragStart(e, tab.id)}
					on:dragover={(e) => onDragOver(e, tab.id)}
					on:drop={(e) => onDrop(e, tab.id)}
					on:dragend={onDragEnd}
					role="tab"
					tabindex="0"
					aria-selected={tab.id === activeTabId}
					on:keydown={(e) =>
						e.key === "Enter" && (activeTabId = tab.id)}
				>
					{#if color}
						<span class="tab-group-dot" style="background:{color}"
						></span>
					{/if}
					{#if tab.loading}
						<span class="tab-spinner"></span>
					{:else}
						<svg
							width="12"
							height="12"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="1.8"
							stroke-linecap="round"
							stroke-linejoin="round"
							class="tab-icon"
							><circle cx="12" cy="12" r="10" /><line
								x1="2"
								y1="12"
								x2="22"
								y2="12"
							/><path
								d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"
							/></svg
						>
					{/if}
					<span class="tab-title">{tab.title}</span>
					<button
						class="tab-close"
						on:click|stopPropagation={() => closeTab(tab.id)}
						aria-label="Close tab"
					>
						<svg
							width="10"
							height="10"
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
			{:else}
				<!-- Group pill -->
				{@const group = item}
				<button
					class="group-pill"
					style="--group-color: {group.color}"
					on:click={() => toggleGroupCollapse(group.id)}
					title="{group.collapsed
						? 'Expand'
						: 'Collapse'} group: {group.name}"
				>
					<span class="group-dot" style="background:{group.color}"
					></span>
					<span class="group-name">{group.name}</span>
					<svg
						width="10"
						height="10"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						stroke-linejoin="round"
						style="transition: transform 0.15s; transform: rotate({group.collapsed
							? '-90deg'
							: '0deg'})"
						><polyline points="6 9 12 15 18 9" /></svg
					>
				</button>
			{/if}
		{/each}

		<button
			class="new-tab-btn"
			on:click={() => openTab()}
			title="New tab (Ctrl+T)"
			aria-label="New tab"
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				><line x1="12" y1="5" x2="12" y2="19" /><line
					x1="5"
					y1="12"
					x2="19"
					y2="12"
				/></svg
			>
		</button>
	</div>

	<!-- TOOLBAR (address bar for active tab) -->
	{#if activeTab}
		<div class="toolbar">
			<div class="nav-buttons">
				<button
					class="icon-btn"
					on:click={() => goBack(activeTabId)}
					disabled={activeTab.historyIndex <= 0}
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
					on:click={() => goForward(activeTabId)}
					disabled={activeTab.historyIndex >=
						activeTab.history.length - 1}
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

			<div class="address-bar" class:loading={activeTab.loading}>
				<span class="scheme">sisi://</span>
				<input
					type="text"
					value={activeTab.inputValue}
					on:input={(e) =>
						updateTab(activeTabId, {
							inputValue: sanitize(
								(e.target as HTMLInputElement).value,
							),
						})}
					on:keydown={(e) => {
						if (e.key === "Enter") navigateTab(activeTabId);
					}}
					on:focus={(e) => (e.target as HTMLInputElement).select()}
					placeholder="enter site hash or address"
					spellcheck="false"
					autocomplete="off"
				/>
				{#if activeTab.loading}
					<span class="spinner"></span>
				{:else}
					<button
						class="go-btn"
						on:click={() => navigateTab(activeTabId)}
						disabled={!activeTab.inputValue.trim()}
						aria-label="Go"
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

		<!-- WEBVIEW AREA -->
		<div class="webview-area">
			{#if activeTab.error}
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
						style="color:#d97706"
						><circle cx="12" cy="12" r="10" /><line
							x1="12"
							y1="8"
							x2="12"
							y2="12"
						/><line x1="12" y1="16" x2="12.01" y2="16" /></svg
					>
					<p class="error-msg">{activeTab.error}</p>
					<p class="error-hint">
						Make sure the address is valid and at least one peer is
						seeding it.
					</p>
				</div>
			{:else if !activeTab.gatewayUrl}
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
							style="color:var(--text-3)"
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
					<p class="empty-title">New tab</p>
					<p class="empty-sub">
						Enter a <code>sisi://</code> address or content hash above
					</p>
					<p class="empty-shortcuts">
						<kbd>Ctrl+T</kbd> new tab &nbsp;·&nbsp;
						<kbd>Ctrl+W</kbd> close tab &nbsp;·&nbsp;
						<kbd>Ctrl+Tab</kbd> next tab
					</p>
				</div>
			{:else}
				<iframe
					bind:this={activeIframe}
					src={activeTab.gatewayUrl}
					title="Sisifo site"
					sandbox="allow-scripts allow-same-origin allow-forms allow-modals allow-popups"
				></iframe>
			{/if}
		</div>
	{/if}
</div>

<!-- CONTEXT MENU -->
{#if ctxMenu}
	<div
		class="ctx-menu"
		style="left:{ctxMenu.x}px; top:{ctxMenu.y}px"
		role="menu"
	>
		<button
			class="ctx-item"
			on:click={() => ctxAction("new-tab")}
			role="menuitem"
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				><line x1="12" y1="5" x2="12" y2="19" /><line
					x1="5"
					y1="12"
					x2="19"
					y2="12"
				/></svg
			>
			New tab
		</button>
		<button
			class="ctx-item"
			on:click={() => ctxAction("duplicate")}
			role="menuitem"
		>
			<svg
				width="13"
				height="13"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				><rect x="9" y="9" width="13" height="13" rx="2" /><path
					d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
				/></svg
			>
			Duplicate tab
		</button>
		<div class="ctx-separator"></div>
		{#if tabs.find((t) => t.id === ctxMenu?.tabId)?.groupId}
			<button
				class="ctx-item"
				on:click={() => ctxAction("remove-group")}
				role="menuitem"
			>
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					><circle cx="12" cy="12" r="10" /><line
						x1="4.93"
						y1="4.93"
						x2="19.07"
						y2="19.07"
					/></svg
				>
				Remove from group
			</button>
		{:else}
			<button
				class="ctx-item"
				on:click={() => ctxAction("new-group")}
				role="menuitem"
			>
				<svg
					width="13"
					height="13"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					><circle cx="9" cy="9" r="4" /><circle
						cx="15"
						cy="15"
						r="4"
					/></svg
				>
				Add to new group
			</button>
			{#if groups.length > 0}
				{#each groups as g}
					<button
						class="ctx-item ctx-group-item"
						on:click={() => {
							updateTab(ctxMenu!.tabId, { groupId: g.id });
							closeCtxMenu();
						}}
						role="menuitem"
					>
						<span class="ctx-group-dot" style="background:{g.color}"
						></span>
						Add to "{g.name}"
					</button>
				{/each}
			{/if}
		{/if}
		<div class="ctx-separator"></div>
		<button
			class="ctx-item ctx-danger"
			on:click={() => ctxAction("close")}
			role="menuitem"
		>
			<svg
				width="13"
				height="13"
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
			Close tab
		</button>
	</div>
{/if}

<!-- GROUP CREATION MODAL -->
{#if showGroupModal}
	<div
		class="modal-overlay"
		on:click|self={() => {
			showGroupModal = false;
		}}
	>
		<div class="modal">
			<p class="modal-title">New tab group</p>
			<input
				class="modal-input"
				bind:value={newGroupName}
				placeholder="Group name"
				on:keydown={(e) => e.key === "Enter" && createGroup()}
				autofocus
			/>
			<div class="color-picker">
				{#each GROUP_COLORS as color}
					<button
						class="color-dot"
						class:selected={newGroupColor === color}
						style="background:{color}"
						on:click={() => (newGroupColor = color)}
						aria-label="Color {color}"
					></button>
				{/each}
			</div>
			<div class="modal-actions">
				<button
					class="modal-btn ghost"
					on:click={() => {
						showGroupModal = false;
					}}>Cancel</button
				>
				<button
					class="modal-btn primary"
					on:click={createGroup}
					disabled={!newGroupName.trim()}>Create group</button
				>
			</div>
		</div>
	</div>
{/if}

<style>
	.browser {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--surface-2);
	}

	/* ── TAB BAR ── */
	.tab-bar {
		display: flex;
		align-items: center;
		background: var(--surface);
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
		scrollbar-width: none;
		min-height: 36px;
		padding: 0 4px;
		gap: 2px;
	}
	.tab-bar::-webkit-scrollbar {
		display: none;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 0 8px 0 10px;
		height: 30px;
		min-width: 80px;
		max-width: 200px;
		border-radius: 6px 6px 0 0;
		cursor: pointer;
		user-select: none;
		background: transparent;
		color: var(--text-2);
		font-size: 12px;
		transition:
			background 0.1s,
			color 0.1s;
		flex-shrink: 0;
		position: relative;
		border: 1px solid transparent;
		border-bottom: none;
		margin-bottom: -1px;
	}
	.tab:hover {
		background: var(--surface-2);
		color: var(--text);
	}
	.tab.active {
		background: var(--surface-2);
		color: var(--text);
		border-color: var(--border);
		border-bottom-color: var(--surface-2);
		z-index: 1;
	}
	.tab.drag-over {
		background: var(--accent-light);
	}

	/* Group color indicator bar at top of tab */
	.tab[style*="--tab-group-color"]::before {
		content: "";
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 2px;
		background: var(--tab-group-color);
		border-radius: 6px 6px 0 0;
	}

	.tab-group-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.tab-icon {
		opacity: 0.4;
		flex-shrink: 0;
	}
	.tab.active .tab-icon {
		opacity: 0.7;
	}

	.tab-spinner {
		width: 12px;
		height: 12px;
		border: 1.5px solid var(--border);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		flex-shrink: 0;
	}

	.tab-title {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 12px;
	}

	.tab-close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 3px;
		cursor: pointer;
		opacity: 0;
		transition:
			opacity 0.1s,
			background 0.1s,
			color 0.1s;
		flex-shrink: 0;
		padding: 0;
	}
	.tab:hover .tab-close,
	.tab.active .tab-close {
		opacity: 1;
	}
	.tab-close:hover {
		background: var(--surface);
		color: var(--text);
		opacity: 1;
	}

	/* ── GROUP PILL ── */
	.group-pill {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 3px 9px;
		height: 22px;
		border-radius: 11px;
		border: 1px solid
			color-mix(in srgb, var(--group-color) 40%, transparent);
		background: color-mix(in srgb, var(--group-color) 12%, transparent);
		color: var(--group-color);
		font-size: 11px;
		font-weight: 500;
		cursor: pointer;
		flex-shrink: 0;
		transition: background 0.15s;
		margin: 0 2px;
		font-family: "DM Sans", sans-serif;
	}
	.group-pill:hover {
		background: color-mix(in srgb, var(--group-color) 20%, transparent);
	}

	.group-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.group-name {
		font-size: 11px;
	}

	.new-tab-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border: none;
		background: none;
		color: var(--text-3);
		border-radius: 4px;
		cursor: pointer;
		flex-shrink: 0;
		margin-left: 2px;
		transition:
			background 0.1s,
			color 0.1s;
	}
	.new-tab-btn:hover {
		background: var(--surface-2);
		color: var(--text);
	}

	/* ── TOOLBAR ── */
	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
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
		height: 32px;
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
	}

	.address-bar input {
		flex: 1;
		border: none;
		background: none;
		font-family: var(--mono);
		font-size: 12px;
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

	/* ── WEBVIEW ── */
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
	.empty-shortcuts {
		font-size: 12px;
		color: var(--text-3);
		margin-top: 8px;
	}
	kbd {
		font-family: var(--mono);
		font-size: 11px;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 3px;
		padding: 1px 5px;
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

	/* ── CONTEXT MENU ── */
	:global(.ctx-menu) {
		position: fixed;
		z-index: 1000;
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 4px;
		min-width: 180px;
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	:global(.ctx-item) {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 7px 10px;
		border: none;
		background: none;
		color: var(--text-2);
		font-size: 13px;
		font-family: "DM Sans", sans-serif;
		cursor: pointer;
		border-radius: 4px;
		text-align: left;
		width: 100%;
		transition:
			background 0.1s,
			color 0.1s;
	}
	:global(.ctx-item:hover) {
		background: var(--surface-2);
		color: var(--text);
	}
	:global(.ctx-danger) {
		color: #ef4444 !important;
	}
	:global(.ctx-danger:hover) {
		background: #fef2f2 !important;
	}
	:global(.ctx-separator) {
		height: 1px;
		background: var(--border);
		margin: 3px 0;
	}
	:global(.ctx-group-dot) {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
	}
	:global(.ctx-group-item) {
		padding-left: 10px;
	}

	/* ── GROUP MODAL ── */
	:global(.modal-overlay) {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.3);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 500;
		backdrop-filter: blur(4px);
	}
	:global(.modal) {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 20px;
		width: 300px;
		display: flex;
		flex-direction: column;
		gap: 14px;
		box-shadow: 0 8px 40px rgba(0, 0, 0, 0.12);
	}
	:global(.modal-title) {
		font-size: 14px;
		font-weight: 600;
		color: var(--text);
	}
	:global(.modal-input) {
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 8px 10px;
		font-size: 13px;
		font-family: "DM Sans", sans-serif;
		color: var(--text);
		background: var(--surface-2);
		outline: none;
		transition: border-color 0.15s;
	}
	:global(.modal-input:focus) {
		border-color: var(--accent);
	}
	:global(.color-picker) {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}
	:global(.color-dot) {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid transparent;
		cursor: pointer;
		transition:
			transform 0.1s,
			border-color 0.1s;
	}
	:global(.color-dot:hover) {
		transform: scale(1.15);
	}
	:global(.color-dot.selected) {
		border-color: var(--text);
		transform: scale(1.1);
	}
	:global(.modal-actions) {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
	}
	:global(.modal-btn) {
		padding: 7px 14px;
		border-radius: 5px;
		font-size: 13px;
		font-family: "DM Sans", sans-serif;
		font-weight: 500;
		cursor: pointer;
		border: 1px solid var(--border);
		transition: all 0.15s;
	}
	:global(.modal-btn.ghost) {
		background: none;
		color: var(--text-2);
	}
	:global(.modal-btn.ghost:hover) {
		border-color: var(--text-2);
		color: var(--text);
	}
	:global(.modal-btn.primary) {
		background: var(--accent);
		color: white;
		border-color: var(--accent);
	}
	:global(.modal-btn.primary:hover) {
		opacity: 0.9;
	}
	:global(.modal-btn.primary:disabled) {
		opacity: 0.4;
		cursor: default;
	}
</style>
