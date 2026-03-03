# glsl-nb

A browser-based GLSL shader notebook. Write fragment shaders in editable cells with live previews — like a notebook but for shaders.

Built with Rust + WebAssembly + [egui](https://github.com/emilk/egui) + WebGL.

## Features

- **Notebook-style cells** — each cell is an independent shader with its own editor and live preview
- **Live compilation** — shaders recompile as you type with inline error diagnostics
- **Shadertoy-compatible** — write `mainImage(out vec4 fragColor, in vec2 fragCoord)` with `iTime`, `iResolution` uniforms
- **Cell splitting** — type `# name WxH` on a new line to split into a new cell
- **Configurable resolution** — set per-cell resolution via the header (e.g. `# my shader 1920x1080`)
- **GLSL syntax highlighting** — Gruvbox Material Dark theme
- **Arrow key navigation** — move between cells with up/down arrows at boundaries

## Getting Started

### Prerequisites

```
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli
```

Or just run:

```
make install
```

### Development

```
make dev
```

Opens at `http://127.0.0.1:8080` with hot reload.

### Build

```
make build
```

Static assets are output to `dist/` (HTML + JS + WASM) — no server needed.

### Deploy to GitHub Pages

Push to `main` and the included GitHub Actions workflow builds and deploys automatically.

To build locally for Pages:

```
make deploy PUBLIC_URL=/glsl-nb/
```

## Cell Format

Each cell starts with a header line:

```
# name resolution
```

For example:

```glsl
# my cool shader 1280x720
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}
```

### Available Uniforms

| Uniform | Type | Description |
|---|---|---|
| `iTime` | `float` | Time in seconds since load |
| `iResolution` | `vec3` | Viewport resolution (width, height, 1.0) |

## License

MIT
