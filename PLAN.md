# Yokisfy — Plan

Reproductor y descargador de música open source (GPL-3) para Windows y Android, con experiencia tipo Spotify y solo fuentes legales y gratuitas.

## Decisiones

| # | Tema | Decisión |
|---|---|---|
| 1 | Fuentes | Catálogo libre + biblioteca propia |
| 2 | Stack | Tauri v2 |
| 3 | Alcance UX | Inicio, búsqueda, biblioteca, reproductor, cola, playlists + radio/recomendaciones |
| 4 | Calidad de descarga | Elegible por el usuario (FLAC > OGG > MP3 según la fuente) |
| 5 | Sync | Local-first, sin cuenta, LAN/archivo |
| 6 | Licencia | GPL-3.0 |
| 7 | Segundo plano Android | Sí, con un plugin Kotlin mínimo (MediaSession + foreground service) |
| 8 | Biblioteca en el celular | La app de PC funciona como servidor Subsonic (LAN; Tailscale fuera de casa) |
| 9 | Recomendaciones | ListenBrainz + MusicBrainz |
| 10 | MVP | Windows primero, Android en la siguiente fase |
| 12 | UI | Núcleo 100% Rust + UI web en Tauri (Svelte 5 + Vite) |
| 13 | Descargas | `Música/Yokisfy/Artista/Álbum/NN - Título.ext` con tags, portada y licencia embebida |
| 14 | Búsqueda | Unificada, con etiqueta de fuente y calidad |
| 15 | PC | Solo Windows en el MVP |

## Fuentes

| Fuente | API | Clave | Formatos |
|---|---|---|---|
| Jamendo | api.jamendo.com/v3.0 | client_id gratis (devportal.jamendo.com) | MP3 / OGG / FLAC |
| Internet Archive | advancedsearch + metadata | No | FLAC / MP3 / OGG |
| Audius | api.audius.co | No | stream; descarga solo si el artista la habilita |
| Musopen | web | No | FLAC (dominio público) |
| radio-browser.info | REST | No | streams de radio |
| ListenBrainz / MusicBrainz | REST | No (User-Agent, 1 req/s en MB) | metadatos y artistas similares |

No se usan fuentes que rompan DRM ni extracción de YouTube o Spotify. Importar playlists de Spotify solo trae metadatos (título y artista) para buscarlos en las fuentes legales o en la biblioteca local.

## Arquitectura

```
yokisfy/
  crates/core/          Rust: motor de audio, fuentes, biblioteca, descargas, recomendaciones
    audio/              symphonia (decode) + cpal (salida; Oboe en Android), cola, gapless
    sources/            trait Source { search, stream_url, download_formats, metadata }
    library/            SQLite (rusqlite): pistas, álbumes, playlists, historial
    download/           reqwest + lofty (tags/portada/licencia), cola de descargas con reanudación
    recs/               ListenBrainz similar-artists + MusicBrainz tags + historial local
    server/             axum: API Subsonic para que el celular reproduzca la biblioteca de la PC
  src-tauri/            comandos Tauri que exponen el core a la UI
  ui/                   Svelte 5 + Vite
  android-plugin/       Kotlin: MediaSession, notificación, controles de audífonos ↔ core Rust (JNI)
```

## Fases

1. **MVP Windows:** reproductor Rust, cola, búsqueda unificada en Jamendo + Internet Archive, descargas con elección de formato, biblioteca SQLite, playlists, UI completa (inicio, búsqueda, biblioteca, reproductor).
2. **Más fuentes y descubrimiento:** Audius, Musopen, radio-browser, radio automática y recomendaciones con ListenBrainz/MusicBrainz, importación de playlists de Spotify (solo metadatos).
3. **Android:** build móvil de Tauri, plugin Kotlin de segundo plano, `npm run android:apk` → `~/Trabajos/APKs/Yokisfy-APK`, instalación por adb en el Nothing.
4. **PC como servidor:** API Subsonic en axum, el Android la usa como fuente, guía de Tailscale, sync de playlists por LAN.

## Requisitos del usuario

- Crear un `client_id` de Jamendo en devportal.jamendo.com y ponerlo en `.env` (`JAMENDO_CLIENT_ID`).
- Opcional para la fase 4: Tailscale en la PC y en el celular.
