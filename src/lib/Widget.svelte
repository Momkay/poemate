<script lang="ts">
  import type { Snippet } from "svelte";
  import { posOf, setPos, saveSoon, snap } from "./layout.svelte";

  let { id, edit, children }: { id: string; edit: boolean; children: Snippet } = $props();

  let el: HTMLElement;
  let drag: { px: number; py: number; x: number; y: number } | null = null;
  const pos = $derived(posOf(id));
  const scale = $derived(pos?.scale ?? 1);

  function down(e: PointerEvent) {
    if (!edit || e.button !== 0) return;
    if ((e.target as HTMLElement).closest("input, button, textarea")) return;
    const r = el.getBoundingClientRect();
    drag = { px: e.clientX, py: e.clientY, x: pos?.x ?? r.left, y: pos?.y ?? r.top };
    el.setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    const r = el.getBoundingClientRect();
    setPos(id, {
      x: snap(drag.x + e.clientX - drag.px, r.width, window.innerWidth),
      y: snap(drag.y + e.clientY - drag.py, r.height, window.innerHeight),
      scale,
    });
  }

  function up() {
    if (drag) saveSoon();
    drag = null;
  }

  function wheel(e: WheelEvent) {
    if (!edit) return;
    e.preventDefault();
    const r = el.getBoundingClientRect();
    const next = Math.max(0.7, Math.min(1.5, Math.round((scale + (e.deltaY < 0 ? 0.05 : -0.05)) * 100) / 100));
    setPos(id, { x: pos?.x ?? r.left, y: pos?.y ?? r.top, scale: next });
    saveSoon();
  }
</script>

<div
  class="widget"
  class:edit
  class:placed={!!pos}
  bind:this={el}
  style:left={pos ? `${pos.x}px` : undefined}
  style:top={pos ? `${pos.y}px` : undefined}
  style:transform={scale !== 1 ? `scale(${scale})` : undefined}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onwheel={wheel}
>
  {@render children()}
</div>

<style>
  .widget {
    transform-origin: top left;
  }
  .widget.placed {
    position: absolute;
    width: 300px; /* same as the flow column, absolute boxes would shrink-wrap their longest line */
  }
  .widget.edit {
    cursor: move;
    touch-action: none;
  }
  .widget.edit :global(.card) {
    outline: 1px dashed rgba(111, 211, 192, 0.5);
  }
</style>
