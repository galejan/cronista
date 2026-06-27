<script lang="ts">
  /**
   * ProjectConfigForm — unified wizard for new-project setup and editing config.
   *
   * new mode: font → tabs → auto-save → review
   * edit mode: tabs → auto-save → review (font handled by ProjectSettingsDialog)
   */
  import { untrack } from "svelte";
  import { t } from "$lib/i18n.svelte";
  import CaretRight from "phosphor-svelte/lib/CaretRight";
  import CaretLeft from "phosphor-svelte/lib/CaretLeft";
  import CheckCircle from "phosphor-svelte/lib/CheckCircle";
  import X from "phosphor-svelte/lib/X";

  interface VisibleTabs {
    chapters: boolean;
    characters: boolean;
    places: boolean;
    timeline: boolean;
    notes: boolean;
    media: boolean;
  }

  interface ProjectConfig {
    font_family: string;
    visible_tabs: VisibleTabs;
    auto_save_interval_minutes: number;
  }

  let {
    mode = "new",
    initialData = {},
    onComplete,
    onCancel,
  }: {
    mode: "new" | "edit";
    initialData?: Record<string, any>;
    onComplete: (config: ProjectConfig) => void;
    onCancel?: () => void;
  } = $props();

  // ── Step state ──────────────────────────────────────────
  type StepName = "font" | "tabs" | "autosave" | "review";

  const stepsNew: StepName[] = ["font", "tabs", "autosave", "review"];
  const stepsEdit: StepName[] = ["tabs", "autosave", "review"];

  let currentStepIdx = $state(0);
  let steps = $derived(mode === "new" ? stepsNew : stepsEdit);
  let currentStep = $derived(steps[currentStepIdx]);

  // ── Form data state ─────────────────────────────────────
  // untrack: we want the initial snapshot, not reactive rebinding on prop change
  let fontFamily = $state(untrack(() => initialData?.font_family ?? "monospace"));

  let visibleTabs = $state<VisibleTabs>(untrack(() => ({
    chapters: initialData?.visible_tabs?.chapters ?? true,
    characters: initialData?.visible_tabs?.characters ?? true,
    places: initialData?.visible_tabs?.places ?? true,
    timeline: initialData?.visible_tabs?.timeline ?? true,
    notes: initialData?.visible_tabs?.notes ?? true,
    media: initialData?.visible_tabs?.media ?? true,
  })));

  let autoSaveInterval = $state(untrack(() => initialData?.auto_save_interval_minutes ?? 5));

  // ── Derived ─────────────────────────────────────────────
  let canGoNext = $derived.by(() => {
    return currentStepIdx < steps.length - 1;
  });

  let canGoBack = $derived.by(() => {
    return currentStepIdx > 0;
  });

  // ── Navigation ──────────────────────────────────────────
  function goNext() {
    if (canGoNext) currentStepIdx++;
  }

  function goBack() {
    if (canGoBack) currentStepIdx--;
  }

  function handleConfirm() {
    onComplete({
      font_family: fontFamily,
      visible_tabs: { ...visibleTabs },
      auto_save_interval_minutes: autoSaveInterval,
    });
  }

  // ── Font options ────────────────────────────────────────
  const fontOptions: { value: string; label: string; cssClass: string }[] = [
    { value: "monospace", label: t("dialog.fontMono"), cssClass: "font-mono" },
    { value: "serif", label: t("dialog.fontSerif"), cssClass: "font-serif" },
    { value: "sans-serif", label: t("dialog.fontSans"), cssClass: "font-sans" },
  ];

  // ── Tab checkbox definitions ────────────────────────────
  interface TabCheckbox {
    key: keyof VisibleTabs;
    label: string;
    disabled: boolean;
  }

  const tabCheckboxes: TabCheckbox[] = [
    { key: "chapters", label: t("config.tabsChapters"), disabled: true },
    { key: "characters", label: t("config.tabsCharacters"), disabled: false },
    { key: "places", label: t("config.tabsPlaces"), disabled: false },
    { key: "timeline", label: t("config.tabsTimeline"), disabled: false },
    { key: "notes", label: t("config.tabsNotes"), disabled: false },
    { key: "media", label: t("config.tabsMedia"), disabled: false },
  ];

  // ── Interval options ────────────────────────────────────
  const intervalOptions: { value: number; label: string }[] = [
    { value: 1, label: t("config.interval1") },
    { value: 5, label: t("config.interval5") },
    { value: 10, label: t("config.interval10") },
  ];
</script>

<div class="config-form">
  <!-- Progress indicator -->
  <div class="progress-bar">
    {#each steps as step, i}
      <div
        class="progress-step"
        class:active={i === currentStepIdx}
        class:done={i < currentStepIdx}
      >
        <span class="step-dot">
          {#if i < currentStepIdx}
            <CheckCircle size={14} weight="fill" />
          {:else}
            {i + 1}
          {/if}
        </span>
        <span class="step-label">
          {#if step === "font"}
            {mode === "new" ? t("config.stepInfo") : t("config.fontLabel")}
          {:else if step === "tabs"}
            {t("config.stepTabs")}
          {:else if step === "autosave"}
            {t("config.stepAutoSave")}
          {:else}
            {t("config.stepReview")}
          {/if}
        </span>
      </div>
      {#if i < steps.length - 1}
        <div class="progress-line" class:done={i < currentStepIdx}></div>
      {/if}
    {/each}
  </div>

  <!-- Step content -->
  <div class="step-body">
    <!-- ═══ Font / Info step ═══ -->
    {#if currentStep === "font"}
      <h3 class="step-title">{t("dialog.fontTitle")}</h3>
      <p class="step-desc">{t("dialog.fontDesc")}</p>

      <fieldset class="font-radio-group">
        <legend class="sr-only">{t("config.fontLabel")}</legend>
        {#each fontOptions as opt}
          <label class="font-radio-label">
            <input
              type="radio"
              name="font-family"
              value={opt.value}
              bind:group={fontFamily}
            />
            <span class="font-radio-text {opt.cssClass}">{opt.label}</span>
            <span class="font-radio-hint">
              {#if opt.value === "monospace"}
                {t("dialog.fontMonoHint")}
              {:else if opt.value === "serif"}
                {t("dialog.fontSerifHint")}
              {:else}
                {t("dialog.fontSansHint")}
              {/if}
            </span>
          </label>
        {/each}
      </fieldset>

      <div class="font-preview-block {fontFamily === 'monospace' ? 'font-mono' : fontFamily === 'serif' ? 'font-serif' : 'font-sans'}">
        <span class="preview-label">{t("settings.fontPreview")}</span>
        <p class="preview-text">
          El viejo coronel se desabrochó el cuello, apoyó el bastón entre las piernas y dijo: «No me parece que haya motivos para alarmarse».
        </p>
      </div>
    {/if}

    <!-- ═══ Tabs step ═══ -->
    {#if currentStep === "tabs"}
      <h3 class="step-title">{t("config.tabsLabel")}</h3>
      <p class="step-desc">{t("config.tabsHint")}</p>

      <div class="tabs-checklist">
        {#each tabCheckboxes as tab}
          <label class="tab-checkbox-label" class:disabled={tab.disabled}>
            <input
              type="checkbox"
              checked={visibleTabs[tab.key]}
              disabled={tab.disabled}
              onchange={() => {
                if (!tab.disabled) {
                  visibleTabs[tab.key] = !visibleTabs[tab.key];
                }
              }}
            />
            <span class="tab-checkbox-text">{tab.label}</span>
          </label>
        {/each}
      </div>
    {/if}

    <!-- ═══ Auto-save step ═══ -->
    {#if currentStep === "autosave"}
      <h3 class="step-title">{t("config.intervalLabel")}</h3>
      <p class="step-desc">{t("config.intervalHint")}</p>

      <fieldset class="interval-radio-group">
        <legend class="sr-only">{t("config.intervalLabel")}</legend>
        {#each intervalOptions as opt}
          <label class="interval-radio-label">
            <input
              type="radio"
              name="auto-save-interval"
              value={opt.value}
              checked={autoSaveInterval === opt.value}
              onchange={() => autoSaveInterval = opt.value}
            />
            <span class="interval-radio-text">{opt.label}</span>
          </label>
        {/each}
      </fieldset>
    {/if}

    <!-- ═══ Review step ═══ -->
    {#if currentStep === "review"}
      <h3 class="step-title">{t("config.reviewTitle")}</h3>

      <div class="review-summary">
        {#if mode === "new"}
          <div class="review-row">
            <span class="review-label">{t("config.reviewFont")}</span>
            <span class="review-value">{fontFamily}</span>
          </div>
        {/if}
        <div class="review-row">
          <span class="review-label">{t("config.reviewTabs")}</span>
          <div class="review-tabs-list">
            {#each tabCheckboxes as tab}
              <span class="review-tab-item" class:hidden={!visibleTabs[tab.key]}>
                {tab.key === "chapters" ? t("tabs.chapters") :
                 tab.key === "characters" ? t("tabs.characters") :
                 tab.key === "places" ? t("tabs.places") :
                 tab.key === "timeline" ? t("tabs.timeline") :
                 tab.key === "media" ? t("tabs.media") :
                 t("tabs.notes")}
                <span class="review-tab-status">
                  {visibleTabs[tab.key] ? t("config.reviewShown") : t("config.reviewHidden")}
                </span>
              </span>
            {/each}
          </div>
        </div>
        <div class="review-row">
          <span class="review-label">{t("config.reviewInterval")}</span>
          <span class="review-value">
            {t("config.reviewIntervalValue").replace("{minutes}", String(autoSaveInterval))}
          </span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Navigation buttons -->
  <div class="step-nav">
    <div class="step-nav-left">
      {#if onCancel}
        <button class="btn-secondary" onclick={onCancel}>
          <X size={14} weight="light" /> {t("config.cancel")}
        </button>
      {/if}
    </div>
    <div class="step-nav-right">
      {#if canGoBack}
        <button class="btn-secondary" onclick={goBack}>
          <CaretLeft size={14} weight="light" /> {t("config.back")}
        </button>
      {/if}
      {#if canGoNext}
        <button class="btn-primary" onclick={goNext}>
          {#if currentStep === "autosave"}
            {t("config.next")} <CaretRight size={14} weight="light" />
          {:else}
            {t("config.next")} <CaretRight size={14} weight="light" />
          {/if}
        </button>
      {:else}
        <button class="btn-primary btn-confirm" onclick={handleConfirm}>
          <CheckCircle size={14} weight="light" /> {t("config.confirm")}
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .config-form {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* ── Progress bar ─────────────────────────────── */
  .progress-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    padding: 0 1rem;
  }

  .progress-step {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
    flex: 0 0 auto;
  }

  .step-dot {
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: 600;
    background: #e2e8f0;
    color: var(--text-muted);
    transition: all 200ms;
  }

  :global(.dark) .step-dot {
    background: #334155;
    color: var(--text-muted);
  }

  .progress-step.active .step-dot {
    background: #3b82f6;
    color: #ffffff;
  }

  .progress-step.done .step-dot {
    background: #22c55e;
    color: #ffffff;
  }

  .step-label {
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    text-align: center;
    max-width: 5rem;
  }

  .progress-step.active .step-label {
    color: #3b82f6;
    font-weight: 600;
  }

  .progress-line {
    width: 1.5rem;
    height: 2px;
    background: #e2e8f0;
    margin: 0 0.25rem;
    margin-bottom: 1rem;
    transition: background 200ms;
  }

  :global(.dark) .progress-line {
    background: #334155;
  }

  .progress-line.done {
    background: #22c55e;
  }

  /* ── Step body ──────────────────────────────────── */
  .step-body {
    background: var(--bg-app);
    border-radius: 0.5rem;
    padding: 1.25rem;
    border: 1px solid var(--border-color);
  }

  :global(.dark) .step-body {
    background: #0f172a;
    border-color: #334155;
  }

  .step-title {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-main);
  }

  :global(.dark) .step-title {
    color: var(--text-title);
  }

  .step-desc {
    margin: 0 0 1rem;
    font-size: 0.8125rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  /* ── Font picker ────────────────────────────────── */
  .font-radio-group {
    border: none;
    padding: 0;
    margin: 0 0 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .font-radio-label {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    padding: 0.625rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    cursor: pointer;
    transition: border-color 150ms, background 150ms;
  }

  .font-radio-label:has(input:checked) {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  :global(.dark) .font-radio-label {
    border-color: #334155;
  }

  :global(.dark) .font-radio-label:has(input:checked) {
    border-color: #60a5fa;
    background: #1e3a5f;
  }

  .font-radio-label input[type="radio"] {
    display: none;
  }

  .font-radio-text {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-main);
  }

  :global(.dark) .font-radio-text {
    color: var(--text-title);
  }

  .font-radio-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .font-mono {
    font-family: ui-monospace, "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
  }

  .font-serif {
    font-family: "Lora", "Merriweather", "Source Serif 4", "IBM Plex Serif", Georgia, serif;
  }

  .font-sans {
    font-family: "Inter", "Roboto", "Open Sans", system-ui, sans-serif;
  }

  .font-preview-block {
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-app);
  }

  :global(.dark) .font-preview-block {
    background: #1e293b;
    border-color: #334155;
  }

  .preview-label {
    display: block;
    font-size: 0.625rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
  }

  .preview-text {
    margin: 0;
    font-size: 0.9375rem;
    line-height: 1.7;
    color: var(--text-main);
  }

  :global(.dark) .preview-text {
    color: var(--text-title);
  }

  /* ── Tabs checklist ─────────────────────────────── */
  .tabs-checklist {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tab-checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    cursor: pointer;
    transition: border-color 150ms, background 150ms;
  }

  :global(.dark) .tab-checkbox-label {
    border-color: #334155;
  }

  .tab-checkbox-label.disabled {
    opacity: 0.7;
    cursor: not-allowed;
    background: var(--bg-active-tab);
  }

  :global(.dark) .tab-checkbox-label.disabled {
    background: #1e293b;
  }

  .tab-checkbox-label input[type="checkbox"] {
    accent-color: #3b82f6;
    width: 1rem;
    height: 1rem;
    cursor: pointer;
  }

  .tab-checkbox-label.disabled input[type="checkbox"] {
    cursor: not-allowed;
  }

  .tab-checkbox-text {
    font-size: 0.875rem;
    color: var(--text-main);
  }

  :global(.dark) .tab-checkbox-text {
    color: var(--text-title);
  }

  /* ── Interval radio group ───────────────────────── */
  .interval-radio-group {
    border: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .interval-radio-label {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.625rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 0.5rem;
    cursor: pointer;
    transition: border-color 150ms, background 150ms;
  }

  .interval-radio-label:has(input:checked) {
    border-color: #3b82f6;
    background: #eff6ff;
  }

  :global(.dark) .interval-radio-label {
    border-color: #334155;
  }

  :global(.dark) .interval-radio-label:has(input:checked) {
    border-color: #60a5fa;
    background: #1e3a5f;
  }

  .interval-radio-label input[type="radio"] {
    accent-color: #3b82f6;
  }

  .interval-radio-text {
    font-size: 0.9375rem;
    color: var(--text-main);
  }

  :global(.dark) .interval-radio-text {
    color: var(--text-title);
  }

  /* ── Review summary ─────────────────────────────── */
  .review-summary {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .review-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border-color);
  }

  :global(.dark) .review-row {
    border-bottom-color: #334155;
  }

  .review-row:last-child {
    border-bottom: none;
  }

  .review-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    min-width: 6rem;
  }

  .review-value {
    font-size: 0.875rem;
    color: var(--text-main);
    text-align: right;
  }

  :global(.dark) .review-value {
    color: var(--text-title);
  }

  .review-tabs-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .review-tab-item {
    font-size: 0.8125rem;
    color: var(--text-main);
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  :global(.dark) .review-tab-item {
    color: var(--text-title);
  }

  .review-tab-item.hidden {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .review-tab-status {
    font-size: 0.6875rem;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  /* ── Navigation ─────────────────────────────────── */
  .step-nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .step-nav-left,
  .step-nav-right {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  /* ── Buttons ────────────────────────────────────── */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 1.25rem;
    border: none;
    border-radius: 0.375rem;
    background: #3b82f6;
    color: #ffffff;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-confirm {
    background: #22c55e;
  }

  .btn-confirm:hover {
    background: #16a34a;
  }

  .btn-secondary {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.5rem 1.25rem;
    border: 1px solid var(--border-color);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms, border-color 150ms;
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-app);
    border-color: #cbd5e1;
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  :global(.dark) .btn-secondary {
    border-color: #334155;
    color: #cbd5e1;
  }

  :global(.dark) .btn-secondary:hover:not(:disabled) {
    background: #334155;
    border-color: var(--text-muted);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
