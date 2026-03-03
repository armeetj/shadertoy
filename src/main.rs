use eframe::egui;
use glow::HasContext;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use web_time::Instant;

// Gruvbox Material Dark Hard
const BG: egui::Color32 = egui::Color32::from_rgb(0x1d, 0x20, 0x21);
const BG1: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);
const FG: egui::Color32 = egui::Color32::from_rgb(0xd4, 0xbe, 0x98);
const RED: egui::Color32 = egui::Color32::from_rgb(0xea, 0x69, 0x62);
const GREEN: egui::Color32 = egui::Color32::from_rgb(0xa9, 0xb6, 0x65);
const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xd8, 0xa6, 0x57);
const BLUE: egui::Color32 = egui::Color32::from_rgb(0x7d, 0xae, 0xa3);
const PURPLE: egui::Color32 = egui::Color32::from_rgb(0xd3, 0x86, 0x9b);
const AQUA: egui::Color32 = egui::Color32::from_rgb(0x89, 0xb4, 0x82);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xe7, 0x8a, 0x4e);
const GRAY: egui::Color32 = egui::Color32::from_rgb(0x92, 0x83, 0x74);

const VERTEX_SHADER: &str = "#version 300 es\n\
precision highp float;\n\
void main() {\n\
    float x = float((gl_VertexID & 1) << 2) - 1.0;\n\
    float y = float((gl_VertexID & 2) << 1) - 1.0;\n\
    gl_Position = vec4(x, y, 0.0, 1.0);\n\
}\n";

const FRAG_PREAMBLE: &str = "#version 300 es\n\
precision highp float;\n\
uniform float iTime;\n\
uniform vec3 iResolution;\n\
uniform vec2 iOffset;\n\
out vec4 fragColor;\n";

const FRAG_POSTAMBLE: &str =
    "\nvoid main() { mainImage(fragColor, gl_FragCoord.xy - iOffset); }\n";

fn default_source(id: usize) -> String {
    format!(
        "# shader {id} 1280x720\n\
void mainImage(out vec4 fragColor, in vec2 fragCoord) {{\n\
    vec2 uv = fragCoord / iResolution.xy;\n\
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);\n\
}}"
    )
}

// --- Header parsing ---

fn parse_header(source: &str) -> (f32, f32) {
    let first_line = source.lines().next().unwrap_or("");
    if let Some(rest) = first_line.strip_prefix("# ") {
        // Last token should be WxH
        if let Some(res) = rest.rsplit(' ').next() {
            if let Some((w, h)) = res.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>()) {
                    return (w.clamp(64.0, 3840.0), h.clamp(64.0, 2160.0));
                }
            }
        }
    }
    (1280.0, 720.0)
}

fn shader_code(source: &str) -> &str {
    source.find('\n').map(|i| &source[i + 1..]).unwrap_or("")
}

// --- Syntax highlighting ---

fn highlight(text: &str, font_id: &egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut first_line = true;
    let mut at_line_start = true;

    while i < len {
        // Header line (first line starting with #)
        if first_line && at_line_start && bytes[i] == b'#' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push(&mut job, &text[start..i], ORANGE, font_id);
            first_line = false;
            at_line_start = true;
            continue;
        }

        first_line = false;

        // Line comment
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push(&mut job, &text[start..i], GRAY, font_id);
            continue;
        }

        // Block comment
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            push(&mut job, &text[start..i], GRAY, font_id);
            at_line_start = false;
            continue;
        }

        // Preprocessor (# at line start, not first line)
        if at_line_start && bytes[i] == b'#' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            push(&mut job, &text[start..i], AQUA, font_id);
            at_line_start = false;
            continue;
        }

        // Newline
        if bytes[i] == b'\n' {
            push(&mut job, "\n", FG, font_id);
            i += 1;
            at_line_start = true;
            continue;
        }

        // Whitespace
        if bytes[i].is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i] != b'\n' && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            push(&mut job, &text[start..i], FG, font_id);
            continue;
        }

        at_line_start = false;

        // Number
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.') {
                i += 1;
            }
            push(&mut job, &text[start..i], PURPLE, font_id);
            continue;
        }

        // Word
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &text[start..i];
            push(&mut job, word, classify_word(word), font_id);
            continue;
        }

        // Punctuation
        push(&mut job, &text[i..i + 1], FG, font_id);
        i += 1;
    }

    job
}

fn push(job: &mut egui::text::LayoutJob, text: &str, color: egui::Color32, font_id: &egui::FontId) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}

fn classify_word(word: &str) -> egui::Color32 {
    match word {
        "void" | "if" | "else" | "for" | "while" | "do" | "return" | "break" | "continue"
        | "discard" | "struct" | "in" | "out" | "inout" | "uniform" | "const" | "precision"
        | "highp" | "mediump" | "lowp" | "true" | "false" | "switch" | "case" | "default" => {
            ORANGE
        }

        "float" | "int" | "uint" | "bool" | "vec2" | "vec3" | "vec4" | "ivec2" | "ivec3"
        | "ivec4" | "bvec2" | "bvec3" | "bvec4" | "uvec2" | "uvec3" | "uvec4" | "mat2"
        | "mat3" | "mat4" | "sampler2D" | "samplerCube" => YELLOW,

        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "pow" | "exp" | "log" | "exp2"
        | "log2" | "sqrt" | "inversesqrt" | "abs" | "sign" | "floor" | "ceil" | "fract"
        | "mod" | "min" | "max" | "clamp" | "mix" | "step" | "smoothstep" | "length"
        | "distance" | "dot" | "cross" | "normalize" | "reflect" | "refract" | "texture"
        | "texelFetch" | "mainImage" | "dFdx" | "dFdy" | "fwidth" | "radians" | "degrees"
        | "lessThan" | "greaterThan" | "equal" | "notEqual" | "any" | "all" | "not" => BLUE,

        "iTime" | "iResolution" | "iOffset" | "fragColor" | "fragCoord" | "gl_FragCoord"
        | "gl_VertexID" => RED,

        _ => FG,
    }
}

// --- GL helpers ---

fn compile_shader(gl: &glow::Context, ty: u32, src: &str) -> Result<glow::Shader, String> {
    unsafe {
        let s = gl.create_shader(ty).map_err(|e| e.to_string())?;
        gl.shader_source(s, src);
        gl.compile_shader(s);
        if gl.get_shader_compile_status(s) {
            Ok(s)
        } else {
            let log = gl.get_shader_info_log(s);
            gl.delete_shader(s);
            Err(log)
        }
    }
}

fn link_program(
    gl: &glow::Context,
    v: glow::Shader,
    f: glow::Shader,
) -> Result<glow::Program, String> {
    unsafe {
        let p = gl.create_program().map_err(|e| e.to_string())?;
        gl.attach_shader(p, v);
        gl.attach_shader(p, f);
        gl.link_program(p);
        gl.detach_shader(p, f);
        if gl.get_program_link_status(p) {
            Ok(p)
        } else {
            let log = gl.get_program_info_log(p);
            gl.delete_program(p);
            Err(log)
        }
    }
}

fn build_frag(user: &str) -> String {
    if user.contains("void main(") {
        format!("{FRAG_PREAMBLE}\n{user}")
    } else {
        format!("{FRAG_PREAMBLE}\n{user}\n{FRAG_POSTAMBLE}")
    }
}

// --- Data model ---

struct ShaderState {
    programs: HashMap<usize, glow::Program>,
    vao: glow::VertexArray,
    time: f32,
}

struct Cell {
    id: usize,
    source: String, // first line: # title WxH, rest: GLSL
    error: Option<String>,
    prev_code: String,
}

struct App {
    cells: Vec<Cell>,
    next_id: usize,
    gl: Arc<glow::Context>,
    vert: glow::Shader,
    shared: Arc<Mutex<ShaderState>>,
    t0: Instant,
}

fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG;
    v.faint_bg_color = BG;

    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.open.corner_radius = egui::CornerRadius::ZERO;

    let no_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.bg_stroke = no_stroke;
    v.widgets.inactive.bg_stroke = no_stroke;
    v.widgets.hovered.bg_stroke = no_stroke;
    v.widgets.active.bg_stroke = no_stroke;

    v.widgets.noninteractive.bg_fill = BG;
    v.widgets.inactive.bg_fill = BG;
    v.widgets.hovered.bg_fill = BG;
    v.widgets.active.bg_fill = BG1;

    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, FG);

    v.selection.bg_fill = egui::Color32::from_rgb(0x3c, 0x38, 0x36);
    v.selection.stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

    style.spacing.item_spacing = egui::vec2(0.0, 0.0);

    ctx.set_style(style);
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc.gl.as_ref().expect("need glow backend").clone();
        let vert =
            compile_shader(&gl, glow::VERTEX_SHADER, VERTEX_SHADER).expect("vert must compile");
        let vao = unsafe { gl.create_vertex_array().unwrap() };

        let shared = Arc::new(Mutex::new(ShaderState {
            programs: HashMap::new(),
            vao,
            time: 0.0,
        }));

        apply_style(&cc.egui_ctx);

        let mut app = Self {
            cells: Vec::new(),
            next_id: 0,
            gl,
            vert,
            shared,
            t0: Instant::now(),
        };
        app.add_cell();
        app
    }

    fn add_cell(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.cells.push(Cell {
            id,
            source: default_source(id),
            error: None,
            prev_code: String::new(),
        });
    }

    fn compile_cell(&mut self, idx: usize) {
        let code = shader_code(&self.cells[idx].source).to_string();
        if code == self.cells[idx].prev_code {
            return;
        }
        let src = build_frag(&code);
        let gl = &self.gl;
        let cell_id = self.cells[idx].id;

        match compile_shader(gl, glow::FRAGMENT_SHADER, &src) {
            Ok(fs) => match link_program(gl, self.vert, fs) {
                Ok(prog) => {
                    let mut state = self.shared.lock().unwrap();
                    if let Some(old) = state.programs.insert(cell_id, prog) {
                        unsafe { gl.delete_program(old) };
                    }
                    drop(state);
                    self.cells[idx].error = None;
                }
                Err(e) => self.cells[idx].error = Some(e),
            },
            Err(e) => self.cells[idx].error = Some(e),
        }
        self.cells[idx].prev_code = code;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = self.t0.elapsed().as_secs_f32();
        self.shared.lock().unwrap().time = time;

        for i in 0..self.cells.len() {
            self.compile_cell(i);
        }

        // Replace Tab key events with 4-space text insertion in the event queue.
        // This runs before any widget processes events, so TextEdit sees "    " as text input.
        ctx.input_mut(|i| {
            let has_tab = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: egui::Key::Tab,
                        pressed: true,
                        ..
                    }
                )
            });
            if has_tab {
                i.events
                    .retain(|e| !matches!(e, egui::Event::Key { key: egui::Key::Tab, .. }));
                i.events.push(egui::Event::Text("    ".into()));
            }
        });

        let mut add = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(16.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let avail_w = ui.available_width();

                    for cell in &mut self.cells {
                        ui.push_id(cell.id, |ui| {
                            let font_id =
                                egui::FontId::new(14.0, egui::FontFamily::Monospace);
                            let font_id_clone = font_id.clone();

                            // Single text editor with syntax highlighting
                            let mut layouter =
                                |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                                    let job = highlight(text, &font_id_clone);
                                    ui.fonts(|f| f.layout_job(job))
                                };

                            let _output = egui::TextEdit::multiline(&mut cell.source)
                                .font(egui::FontId::new(14.0, egui::FontFamily::Monospace))
                                .desired_width(avail_w)
                                .desired_rows(4)
                                .frame(false)
                                .lock_focus(true)
                                .layouter(&mut layouter)
                                .show(ui);

                            // (Tab handled globally via event queue replacement above)

                            // Errors inline as comments
                            if let Some(err) = &cell.error {
                                for line in err.lines() {
                                    let line = line.trim();
                                    if !line.is_empty() {
                                        ui.label(
                                            egui::RichText::new(format!("// {line}"))
                                                .font(font_id.clone())
                                                .color(RED),
                                        );
                                    }
                                }
                            }

                            // Preview
                            let (res_w, res_h) = parse_header(&cell.source);
                            let aspect = res_w / res_h;
                            let display_w = (avail_w * 0.5).min(res_w);
                            let display_h = display_w / aspect;

                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(display_w, display_h),
                                egui::Sense::hover(),
                            );

                            let cell_id = cell.id;
                            let shared = self.shared.clone();

                            let callback = egui::PaintCallback {
                                rect,
                                callback: Arc::new(egui_glow::CallbackFn::new(
                                    move |info, painter| {
                                        let gl = painter.gl();
                                        let state = shared.lock().unwrap();
                                        let Some(program) = state.programs.get(&cell_id)
                                        else {
                                            return;
                                        };
                                        let program = *program;
                                        let vao = state.vao;
                                        let time = state.time;
                                        let vp = info.viewport_in_pixels();
                                        drop(state);

                                        unsafe {
                                            gl.viewport(
                                                vp.left_px,
                                                vp.from_bottom_px,
                                                vp.width_px,
                                                vp.height_px,
                                            );
                                            gl.scissor(
                                                vp.left_px,
                                                vp.from_bottom_px,
                                                vp.width_px,
                                                vp.height_px,
                                            );
                                            gl.enable(glow::SCISSOR_TEST);
                                            gl.use_program(Some(program));
                                            if let Some(loc) =
                                                gl.get_uniform_location(program, "iTime")
                                            {
                                                gl.uniform_1_f32(Some(&loc), time);
                                            }
                                            if let Some(loc) =
                                                gl.get_uniform_location(program, "iResolution")
                                            {
                                                gl.uniform_3_f32(
                                                    Some(&loc),
                                                    res_w,
                                                    res_h,
                                                    1.0,
                                                );
                                            }
                                            if let Some(loc) =
                                                gl.get_uniform_location(program, "iOffset")
                                            {
                                                gl.uniform_2_f32(
                                                    Some(&loc),
                                                    vp.left_px as f32,
                                                    vp.from_bottom_px as f32,
                                                );
                                            }
                                            gl.bind_vertex_array(Some(vao));
                                            gl.draw_arrays(glow::TRIANGLES, 0, 3);
                                            gl.bind_vertex_array(None);
                                            gl.use_program(None);
                                            gl.disable(glow::SCISSOR_TEST);
                                        }
                                    },
                                )),
                            };
                            ui.painter().add(callback);

                        });
                    }

                    // "New" — just clickable text
                    let new_label = ui.label(
                        egui::RichText::new("# new shader")
                            .font(egui::FontId::new(14.0, egui::FontFamily::Monospace))
                            .color(GRAY),
                    );
                    if new_label.clicked() {
                        add = true;
                    }
                    if new_label.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
            });

        if add {
            self.add_cell();
        }

        // Remove empty cells (except the last one)
        if self.cells.len() > 1 {
            let gl = &self.gl;
            let shared = &self.shared;
            self.cells.retain(|c| {
                let keep = !c.source.trim().is_empty();
                if !keep {
                    let mut state = shared.lock().unwrap();
                    if let Some(prog) = state.programs.remove(&c.id) {
                        unsafe { gl.delete_program(prog) };
                    }
                }
                keep
            });
        }

        ctx.request_repaint();
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            unsafe { gl.delete_shader(self.vert) };
            let mut state = self.shared.lock().unwrap();
            unsafe { gl.delete_vertex_array(state.vao) };
            for (_, prog) in state.programs.drain() {
                unsafe { gl.delete_program(prog) };
            }
        }
    }
}

fn main() -> eframe::Result {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
            ..Default::default()
        };
        eframe::run_native(
            "GLSL Notebook",
            options,
            Box::new(|cc| Ok(Box::new(App::new(cc)))),
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        wasm_bindgen_futures::spawn_local(async {
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("the_canvas_id")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            eframe::WebRunner::new()
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(|cc| Ok(Box::new(App::new(cc)))),
                )
                .await
                .expect("failed to start eframe");
        });
        Ok(())
    }
}
