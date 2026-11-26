<script>
  let { name, size = 20, stroke = 1.75 } = $props()
  const P = {
    home: 'M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z',
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4',
    library: 'M4 4v16M9 4v16M14 4.5l5 15',
    download: 'M12 4v11M7 10l5 5 5-5M5 20h14',
    settings: 'M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM19.4 13.5a7.6 7.6 0 0 0 0-3l2-1.5-2-3.4-2.3.9a7.5 7.5 0 0 0-2.6-1.5L14 2.5h-4l-.5 2.5a7.5 7.5 0 0 0-2.6 1.5l-2.3-.9-2 3.4 2 1.5a7.6 7.6 0 0 0 0 3l-2 1.5 2 3.4 2.3-.9a7.5 7.5 0 0 0 2.6 1.5l.5 2.5h4l.5-2.5a7.5 7.5 0 0 0 2.6-1.5l2.3.9 2-3.4z',
    play: 'M7 4.5v15l13-7.5z',
    pause: 'M7 4h3.5v16H7zM13.5 4H17v16h-3.5z',
    next: 'M5 5v14l10-7zM18 5v14',
    prev: 'M19 5v14L9 12zM6 5v14',
    shuffle: 'M3 7h3.5c2 0 3.2 1 4.3 2.6l2.4 4.8C14.3 16 15.5 17 17.5 17H21M18 14l3 3-3 3M3 17h3.5c1.2 0 2.1-.4 2.9-1.1M14.6 8.1C15.4 7.4 16.3 7 17.5 7H21M18 4l3 3-3 3',
    repeat: 'M4 11V9a3 3 0 0 1 3-3h13M17 3l3 3-3 3M20 13v2a3 3 0 0 1-3 3H4M7 21l-3-3 3-3',
    heart: 'M12 20s-7.5-4.6-9-9.2C2 7.4 4.2 4.5 7.4 4.5c2 0 3.6 1.1 4.6 2.7 1-1.6 2.6-2.7 4.6-2.7 3.2 0 5.4 2.9 4.4 6.3-1.5 4.6-9 9.2-9 9.2z',
    more: 'M5 12h.01M12 12h.01M19 12h.01',
    plus: 'M12 5v14M5 12h14',
    queue: 'M4 6h12M4 11h12M4 16h7M15 15.5v-5l5 2.5z',
    list: 'M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01',
    volume: 'M4 9.5h3.5L12 5.5v13l-4.5-4H4zM16 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11',
    mute: 'M4 9.5h3.5L12 5.5v13l-4.5-4H4zM16.5 9.5l5 5M21.5 9.5l-5 5',
    check: 'M5 12.5l4.5 4.5L19 7.5',
    close: 'M6 6l12 12M18 6 6 18',
    back: 'M15 5l-7 7 7 7',
    down: 'M6 9l6 6 6-6',
    sun: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2.5v2M12 19.5v2M4.6 4.6 6 6M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z',
    folder: 'M3 6.5A1.5 1.5 0 0 1 4.5 5H9l2 2.5h8.5A1.5 1.5 0 0 1 21 9v9.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z',
    trash: 'M4 7h16M9 7V4.5h6V7M6.5 7l1 13h9l1-13M10 11v5M14 11v5',
    spark: 'M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8zM19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z',
    lyrics: 'M4 6h16M4 11h10M4 16h7M17 13v7M17 20a2 2 0 1 1-2-2h2M17 13l3-1',
    mic: 'M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7',
    disc: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
    info: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 11v5M12 8h.01',
    edit: 'M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4',
    radio: 'M5 9h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1zM7 9l10-5M9 14.5a1.5 1.5 0 1 0 0 .01M14 13h3M14 16h3',
    expand: 'M6 15l6-6 6 6',
    eq: 'M6 4v16M12 4v16M18 4v16M3.5 14h5M9.5 8h5M15.5 16h5',
    stats: 'M4 20V10M10 20V4M16 20v-7M22 20H2',
    party: 'M4 20l4.5-12L16 15.5zM14 4l1 2M18 8l2-1M17 3.5l-1 2.5M20 12l-2.5.5M11 7.5c1-1 2.5-1 3.5 0',
    remote: 'M8 2.5h8a2 2 0 0 1 2 2v15a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-15a2 2 0 0 1 2-2zM12 6.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM10 16h.01M14 16h.01M12 18.5h.01',
    mini: 'M3 5h18v14H3zM12.5 12.5h6v4.5h-6z',
    sync: 'M4 12a8 8 0 0 1 14-5.3L20 8M20 4v4h-4M20 12a8 8 0 0 1-14 5.3L4 16M4 20v-4h4',
    output: 'M4 5h16v10H4zM9 19h6M12 15v4',
    keyboard: 'M3 6h18v12H3zM7 10h.01M11 10h.01M15 10h.01M7 14h10'
  }
  const filled = { play: 1, pause: 1, heartFill: 1 }
</script>

{#if name === 'heartFill'}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d={P.heart} /></svg>
{:else}
  <svg width={size} height={size} viewBox="0 0 24 24" fill={filled[name] ? 'currentColor' : 'none'} stroke={filled[name] ? 'none' : 'currentColor'} stroke-width={name === 'more' ? 3 : stroke} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={P[name]} /></svg>
{/if}
