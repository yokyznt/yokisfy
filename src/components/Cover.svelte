<script>
  import Icon from './Icon.svelte'
  let { src = null, size = 48, radius = 6, alt = '' } = $props()
  let failed = $state(false)
  $effect(() => {
    src
    failed = false
  })
</script>

<span class="cover" style={`--s:${typeof size === 'number' ? size + 'px' : size};--r:${radius}px`}>
  {#if src && !failed}
    <img {src} {alt} loading="lazy" decoding="async" onerror={() => (failed = true)} draggable="false" />
  {:else}
    <Icon name="disc" size={typeof size === 'number' ? Math.max(16, size * 0.38) : 32} stroke={1.4} />
  {/if}
</span>

<style>
  .cover {
    width: var(--s);
    height: var(--s);
    aspect-ratio: 1;
    border-radius: var(--r);
    background: var(--bg-3);
    color: var(--tx-3);
    display: grid;
    place-items: center;
    overflow: hidden;
    flex: none;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
</style>
