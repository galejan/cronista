<script lang="ts">
  let sidebarVisible = $state(true);

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "b") {
      e.preventDefault();
      sidebarVisible = !sidebarVisible;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="app-layout"
  class:sidebar-collapsed={!sidebarVisible}
>
  <!-- Sidebar (40% when visible) -->
  <aside class="sidebar">
    <nav class="tabs">
      <button class="tab active">Capítulos</button>
      <button class="tab">Personajes</button>
      <button class="tab">Notas</button>
    </nav>
    <div class="sidebar-content">
      <p class="text-gray-500 dark:text-gray-400 text-sm p-4">
        Seleccioná un capítulo de la lista para empezar a editar.
      </p>
    </div>
  </aside>

  <!-- Editor (60% when visible, 100% when sidebar collapsed) -->
  <main class="editor">
    <p class="placeholder">
      Seleccioná un capítulo para empezar a escribir
    </p>
  </main>
</div>

<style>
  .app-layout {
    display: grid;
    grid-template-columns: 40% 60%;
    height: 100vh;
    transition: grid-template-columns 300ms ease;
  }

  .app-layout.sidebar-collapsed {
    grid-template-columns: 0% 100%;
  }

  .sidebar {
    overflow: hidden;
    border-right: 1px solid #e2e8f0;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  @media (prefers-color-scheme: dark) {
    .sidebar {
      border-right-color: #334155;
    }
  }

  .sidebar-collapsed .sidebar {
    border-right: none;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid #e2e8f0;
  }

  @media (prefers-color-scheme: dark) {
    .tabs {
      border-bottom-color: #334155;
    }
  }

  .tab {
    flex: 1;
    padding: 0.75rem 0.5rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: #64748b;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 150ms, border-color 150ms;
  }

  .tab:hover {
    color: #1e293b;
  }

  .tab.active {
    color: #3b82f6;
    border-bottom-color: #3b82f6;
  }

  @media (prefers-color-scheme: dark) {
    .tab {
      color: #94a3b8;
    }
    .tab:hover {
      color: #e2e8f0;
    }
    .tab.active {
      color: #60a5fa;
      border-bottom-color: #60a5fa;
    }
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
  }

  .editor {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    overflow: hidden;
  }

  .placeholder {
    color: #94a3b8;
    font-size: 1.125rem;
    font-style: italic;
    user-select: none;
  }

  @media (prefers-color-scheme: dark) {
    .placeholder {
      color: #64748b;
    }
  }
</style>
