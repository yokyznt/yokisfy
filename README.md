# Yokisfy

Reproductor y descargador de música libre para Windows (Android en camino). Núcleo en Rust e interfaz con Tauri v2 + Svelte 5.

Solo usa fuentes legales: música con licencia Creative Commons o con permiso del artista. Las canciones que el artista marca como "solo streaming" se pueden escuchar, pero no descargar.

## Fuentes

| Fuente | Qué ofrece | Descarga |
|---|---|---|
| Internet Archive | Netlabels y Live Music Archive | FLAC, MP3 o OGG, según lo que publique el artista |
| Jamendo | ~600 000 canciones Creative Commons | FLAC, MP3 u OGG (necesitas un `client_id` gratis) |

## Funciones

- Búsqueda unificada con etiqueta de fuente y calidad
- Reproductor en Rust (symphonia + cpal vía rodio) con cola, aleatorio, repetir y adelantar
- Streaming en FLAC cuando la fuente lo permite
- Descargas organizadas en `Música/Yokisfy/Artista/Álbum/NN - Título.ext`, con etiquetas, portada y licencia dentro del archivo
- Me gusta, playlists, historial y descargas sin conexión (SQLite)
- Tema oscuro y claro, diseño adaptable a celular

## Desarrollo

Requisitos: Rust estable, Node 20+, Visual Studio Build Tools (C++) y WebView2.

```bash
npm install
npm run tauri dev
npm run tauri build
```

Prueba del núcleo sin interfaz: `cargo run -p yokisfy-core --example probe`.

Para Jamendo, crea una app en [devportal.jamendo.com](https://devportal.jamendo.com/) y pega el `client_id` en Ajustes.

## Licencia

GPL-3.0-or-later. Ver [LICENSE](LICENSE).
