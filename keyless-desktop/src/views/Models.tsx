/**
 * Models view component.
 * 
 * Displays the model catalog with download management capabilities.
 * Features:
 * - Model list with download status, size, and actions (download, use, delete, pause/resume/cancel)
 * - Sortable by status or size
 * - Real-time download progress tracking
 * - Auto-refresh on mount (deferred for smooth page transitions)
 * - Scroll fade effect for long lists
 * 
 * Handles all model-related events from the backend and updates UI accordingly.
 */

import { useEffect, useMemo, useRef, useState } from 'react';
import { ScrollFade } from '../components/ui/ScrollFade';
import { useScrollFade } from '../hooks/useScrollFade';
import { useModelEventListeners } from '../hooks/useModelEventListeners';
import { useModelsState } from '../hooks/useModelsState';
import { ModelRow } from '../components/ModelRow';
import { LinkButton } from '../components/buttons/LinkButton';
import type { BackendModelInfo } from '../types';

export default function ModelsView({ onBack, overlayOffset }: { onBack?: () => void; overlayOffset?: number }) {
  // Consolidated model state management (replaces multiple useState hooks).
  const { state, actions } = useModelsState();
  // Reference to scrollable list container (for scroll fade effect).
  const listRef = useRef<HTMLDivElement | null>(null);
  // Calculate fade alpha for bottom scroll fade effect.
  const fadeAlpha = useScrollFade(listRef);
  // Track which model's delete button is being hovered (for visual feedback).
  const [hoverDeleteId, setHoverDeleteId] = useState<string | null>(null);

  // Listen for model-related events from backend (downloads, selection, size updates).
  useModelEventListeners({
    onDownloadStarted: actions.handleDownloadStarted,
    onDownloadStage: actions.handleDownloadStage,
    onDownloadProgress: (modelId, progress) => {
      // Only update progress if progress data is available.
      if (progress) {
        actions.handleDownloadProgress(modelId, progress);
      }
    },
    onDownloadComplete: actions.handleDownloadComplete,
    onDownloadError: actions.handleDownloadError,
    onDownloadCancelled: actions.handleDownloadCancelled,
    onDownloadPaused: actions.handleDownloadPaused,
    onModelSelected: actions.handleModelSelected,
    onModelSizesUpdated: actions.handleModelSizesUpdated,
  });

  /**
   * Defer initial model refresh to allow page transition to start rendering smoothly.
   * 
   * Uses double requestAnimationFrame to ensure the view transition animation starts
   * before triggering the potentially expensive model refresh operation.
   * This prevents jank during view transitions.
   */
  useEffect(() => {
    let raf1 = 0;
    let raf2 = 0;
    const schedule = () => {
      // Double RAF: first frame starts transition, second frame triggers refresh.
      raf1 = requestAnimationFrame(() => {
        raf2 = requestAnimationFrame(() => {
          actions.refreshModels();
        });
      });
    };
    schedule();
    // Cleanup: cancel animation frames if component unmounts.
    return () => {
      if (raf1) cancelAnimationFrame(raf1);
      if (raf2) cancelAnimationFrame(raf2);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // actions.refreshModels is stable

  // Loading state: true when fetching and no models loaded yet.
  const isLoading = state.isFetching && state.models.length === 0;

  /**
   * Compute sorted model rows based on current sort preference.
   * 
   * Sorting options:
   * - 'size': Sort by model size (largest first)
   * - 'status': Sort by download status (downloading first, then downloaded, then available)
   */
  const rows = useMemo(() => {
    let r = state.models;
    // Sort by size: largest models first.
    if (state.sort === 'size') {
      r = [...r].sort((a, b) => (state.modelSizes[b.id] ?? 0) - (state.modelSizes[a.id] ?? 0));
    }
    // Sort by status: downloading (0) > downloaded (1) > available (2), then alphabetically.
    if (state.sort === 'status') {
      r = [...r].sort((a, b) => {
        const pri = (m: BackendModelInfo) => (m.downloading ? 0 : m.downloaded ? 1 : 2);
        return pri(a) - pri(b) || a.id.localeCompare(b.id);
      });
    }
    return r;
  }, [state.models, state.sort, state.modelSizes]);

  return (
    <div className="mx-3 mb-2 rounded-xl border border-border bg-bgCard overflow-hidden">
      <div className="slide-up">
        {/* Header: title and sort controls */}
        <div className="pt-[26px] px-[22px] pb-[14px] flex items-center justify-between">
          <h2 className="text-[16px] font-medium text-textAlt lowercase">models</h2>
          {/* Sort toggle: switch between 'status' and 'size' sorting */}
          <div className="flex gap-2 items-center" role="group" aria-label="sort models">
            <div className="inline-flex items-center bg-bgInput border border-border rounded-full p-[2px]">
              {(['status', 'size'] as const).map((k) => {
                const active = state.sort === k;
                return (
                  <button
                    key={k}
                    aria-pressed={active}
                    onClick={() => actions.setSort(k)}
                    className={`text-[11px] lowercase px-3 py-1 rounded-full ${active ? 'bg-border text-textPrimary' : 'text-textSecondary hover:text-textPrimary'}`}
                  >
                    {k}
                  </button>
                );
              })}
            </div>
          </div>
        </div>
        {/* Model count / loading indicator */}
        <div className="px-[22px] pb-[10px] text-[11px] text-textSecondary lowercase">
          {state.isFetching ? 'loading…' : `${rows.length} models`}
        </div>
        {/* Scrollable model list */}
        <div ref={listRef} className="px-[22px] pb-[16px] space-y-2 scrollbar-hide" style={{ maxHeight: overlayOffset ? `calc(100vh - ${overlayOffset + 200}px)` : 'calc(100vh - 200px)', overflowY: 'auto' }}>
          {isLoading ? (
            // Loading state: show while fetching initial models.
            <div className="py-8 text-center">
              <p className="text-textSecondary lowercase">loading models...</p>
            </div>
          ) : (
            // Model rows: render each model with its status, size, and actions.
            rows.map((m, index) => {
              // Check if this model is currently selected (highlighted in UI).
              const isCurrent = Boolean(state.currentModel) && m.id === state.currentModel;
              return (
                <ModelRow
                  key={m.id}
                  model={m}
                  status={state.modelStatuses[m.id]}
                  size={state.modelSizes[m.id]}
                  isCurrent={isCurrent}
                  index={index}
                  hoverDeleteId={hoverDeleteId}
                  onUse={actions.onUse}
                  onDownload={actions.onDownload}
                  onDelete={actions.onDelete}
                  onResume={actions.onResume}
                  onCancel={actions.onCancel}
                  onHoverDelete={setHoverDeleteId}
                />
              );
            })
          )}
          {/* Bottom fade: visual indicator when list is scrollable */}
          <div className="-mx-[22px]" style={{ position: 'sticky', bottom: -16 }}>
            <ScrollFade alpha={fadeAlpha} className="h-3" />
          </div>
        </div>
        {/* Footer: back button */}
        <div className="sticky bottom-0 inset-x-0 text-center py-[12px]">
          <LinkButton onClick={onBack}>[back]</LinkButton>
        </div>
      </div>
    </div>
  );
}
