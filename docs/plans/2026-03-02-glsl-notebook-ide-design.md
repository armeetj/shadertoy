# GLSL Notebook IDE — Design Document

## Summary

A notebook-style GLSL shader IDE built entirely in Rust, compiled to WASM, targeting the browser. Each cell contains a GLSL fragment shader editor with a live, resizable preview canvas below it. Shaders compile on every keystroke and render at 60fps with animated uniforms.

## Stack

- **Rust** compiled to **WASM** via `wasm-pack` / `trunk`
- **eframe** (egui's web/native framework) for the full GUI
- **glow** (OpenGL ES 3.0 / WebGL2) for shader compilation and rendering
- No JavaScript UI — everything is egui

## Architecture

```
┌──────────────────────────────────────────┐
│  Browser (WASM)                          │
│                                          │
│  eframe / egui                           │
│  ├── App                                 │
│  │   ├── Notebook                        │
│  │   │   ├── Cell 0 [editor + preview]   │
│  │   │   ├── Cell 1 [editor + preview]   │
│  │   │   └── ...                         │
│  │   └── ShaderRenderer (glow)           │
│  │       ├── compile GLSL per cell       │
│  │       ├── render to FBO textures      │
│  │       └── hand texture IDs to egui    │
│  └── eframe GL context (shared)          │
└──────────────────────────────────────────┘
```

## Data Model

```rust
struct App {
    notebook: Notebook,
    renderer: ShaderRenderer,
}

struct Notebook {
    cells: Vec<Cell>,
}

struct Cell {
    id: usize,
    source: String,              // GLSL source code
    preview_size: Vec2,          // user-resizable, default 512x512
    compile_status: CompileStatus,
}

enum CompileStatus {
    Ok,
    Error(String),
    Empty,
}

struct ShaderRenderer {
    gl: Arc<glow::Context>,
    cells: HashMap<usize, CellRenderState>,
}

struct CellRenderState {
    program: Option<glow::Program>,
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    egui_texture_id: egui::TextureId,
    last_good_program: Option<glow::Program>,
}
```

## Rendering Pipeline

### Fragment shader wrapper

Users write `mainImage` (ShaderToy convention). We wrap it:

```glsl
#version 300 es
precision highp float;

uniform float iTime;
uniform vec3 iResolution;
out vec4 fragColor;

// ---- user code ----

void main() {
    mainImage(fragColor, gl_FragCoord.xy);
}
```

If the user writes a complete shader with `void main()`, we detect that and skip the wrapper.

### Per-frame render loop

1. eframe calls `App::update()` each frame
2. For each cell with a valid program:
   - Bind cell's FBO
   - Set viewport to cell's preview size
   - Bind shader program
   - Set uniforms: `iTime`, `iResolution`
   - Draw fullscreen quad (2 triangles)
   - Unbind FBO
3. egui renders UI, displaying each FBO texture as `egui::Image`

### Shader compilation

- Compiles on **every keystroke** (no debounce — WebGL compilation is fast)
- On error: show error text in red, keep rendering `last_good_program`
- On success: swap in the new program, drop the old one

## Uniforms (v1)

| Uniform | Type | Description |
|---------|------|-------------|
| `iTime` | `float` | Seconds since cell creation |
| `iResolution` | `vec3` | `(width, height, 1.0)` of preview |
| `gl_FragCoord` | built-in | Fragment coordinate |

## UX & Interaction

- **Layout**: Vertical scrollable list of cells (dark theme)
- **Cell**: Code editor (`TextEdit::multiline`) on top, resizable preview below
- **New cell**: Button at bottom + Ctrl+Enter shortcut
- **Delete cell**: "x" button on hover
- **Resize preview**: Drag handle at bottom-right corner
- **Error display**: Red text below editor with GLSL compiler error
- **Default shader**: New cells start with an animated color gradient example

### Default shader

```glsl
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}
```

## Out of Scope (v1)

- Save/load
- Shader input textures (iChannel0 etc)
- iMouse uniform
- Multi-pass rendering
- Sharing/export
- Syntax highlighting (egui TextEdit is plain text)

## Key Dependencies

- `eframe` — egui web framework
- `glow` — OpenGL bindings (used by eframe internally, we share the context)
- `web-sys` / `js-sys` — WASM web bindings (for timer, etc.)
- `trunk` — WASM build tool and dev server
