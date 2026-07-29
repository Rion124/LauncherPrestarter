<script lang="ts">
  import ProgressBar from "./ProgressBar.svelte";

  export let error;
  export let speedMb;
  export let percentage: number;
  export let totalLabel = "";
</script>

<div data-tauri-drag-region class="download-block">
  <div class="speed-block">
    <div>
      {#if speedMb !== ""}
        <strong class={error ? "errored" : ""}>{speedMb}</strong>
        <small>Mbps</small>
      {:else}
        <strong>--</strong>
      {/if}
    </div>
    <small>{totalLabel}</small>
  </div>
  <ProgressBar class={error ? "errored" : ""} {percentage} />
</div>
{#if error !== null}
    <div class="error-label">
    {error}
  </div>
{/if}

<style lang="scss">
  @use "sass:math";
  .download-block {
    display: flex;
    flex-direction: row;
    justify-content: center;
    align-items: center;
  }
  .speed-block {
    display: flex;
    width: 5rem;
    height: 72px;
    flex-direction: column;
    flex-wrap: nowrap;
    justify-content: center;
    align-items: center;
    gap: 0.375rem;
    // Card styled like the launcher's panels: dark fill, faint ember rim.
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid $stroke;
    border-radius: 0.6rem;
    margin-right: 0.75rem;
    box-shadow: 0 6px 16px $shadow;
    position: relative;

    > div {
      flex-direction: column;
      display: flex;
      align-items: center;

      > strong {
        font-size: 1.5rem;
        font-weight: 800;
        letter-spacing: 0.16px;
        color: transparent;
        background: $active;
        background-clip: text;
        text-shadow: none;
        text-align: center;
        &.errored {
          color: $error;
        }
      }

      > small {
        font-size: 0.75rem;
        color: $text-description;
        text-align: center;
      }
    }

    > small {
      font-size: 0.75rem;
      color: $text-secondary;
    }
  }
  .error-label {
    color: $error;
    text-align: center;
    font-size: 0.75rem;
  }
</style>
