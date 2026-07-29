<script lang="ts">
  import { onDestroy } from "svelte";

  export let percentage: number = 0;

  onDestroy(() => (percentage = 0));
</script>

<div class={"progress-bar " + ($$restProps.class || "")}>
  <div style="width: {percentage}%; height: 100%;"></div>
</div>

<style lang="scss">
  .progress-bar {
    width: 24rem;
    height: 30px;
    background: $base;
    border: 1px solid $stroke;
    border-radius: 0.4rem;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    position: relative;
    > div {
      background: $progressbar;
      // Ember bar glows the way the launcher's "ИГРАТЬ" button does.
      box-shadow: 0 0 12px rgba(242, 135, 30, 0.55);
      transition: width 0.5s cubic-bezier(0.22, 0.61, 0.36, 1);
    }
    &.errored {
      background: $error;
    }
  }
</style>
