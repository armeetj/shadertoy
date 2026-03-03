use eframe::egui;
use glow::HasContext;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use web_time::Instant;

const DEFAULT_SHADER: &str = r#"void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}"#;

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
out vec4 fragColor;\n";

const FRAG_POSTAMBLE: &str = "\nvoid main() { mainImage(fragColor, gl_FragCoord.xy); }\n";

// --- Shared GL state accessible from paint callbacks ---

struct ShaderState {
    programs: HashMap<usize, glow::Program>,
    vao: glow::VertexArray,
    time: f32,
}

// --- Data model ---

struct Cell {
    id: usize,
    source: String,
    preview_height: f32,
    error: Option<String>,
    prev_source: String,
}

struct App {
    cells: Vec<Cell>,
    next_id: usize,
    gl: Arc<glow::Context>,
    vert: glow::Shader,
    shared: Arc<Mutex<ShaderState>>,
    t0: Instant,
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

// --- App ---

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
            source: DEFAULT_SHADER.into(),
            preview_height: 300.0,
            error: None,
            prev_source: String::new(),
        });
    }

    fn compile_cell(&mut self, idx: usize) {
        let cell = &self.cells[idx];
        if cell.source == cell.prev_source {
            return;
        }
        let src = build_frag(&cell.source);
        let gl = &self.gl;

        match compile_shader(gl, glow::FRAGMENT_SHADER, &src) {
            Ok(fs) => match link_program(gl, self.vert, fs) {
                Ok(prog) => {
                    let mut state = self.shared.lock().unwrap();
                    if let Some(old) = state.programs.insert(cell.id, prog) {
                        unsafe { gl.delete_program(old) };
                    }
                    drop(state);
                    self.cells[idx].error = None;
                }
                Err(e) => {
                    self.cells[idx].error = Some(e);
                }
            },
            Err(e) => {
                self.cells[idx].error = Some(e);
            }
        }
        self.cells[idx].prev_source = self.cells[idx].source.clone();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = self.t0.elapsed().as_secs_f32();
        self.shared.lock().unwrap().time = time;

        // Compile changed shaders
        for i in 0..self.cells.len() {
            self.compile_cell(i);
        }

        let mut remove_id = None;
        let mut add = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let w = ui.available_width();

                for cell in &mut self.cells {
                    ui.push_id(cell.id, |ui| {
                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("[{}]", cell.id))
                                    .monospace()
                                    .color(egui::Color32::from_rgb(100, 100, 140)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("x").clicked() {
                                        remove_id = Some(cell.id);
                                    }
                                },
                            );
                        });

                        // Code editor
                        ui.add(
                            egui::TextEdit::multiline(&mut cell.source)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(w)
                                .desired_rows(6),
                        );

                        // Error
                        if let Some(err) = &cell.error {
                            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), err);
                        }

                        // Preview via PaintCallback
                        let h = cell.preview_height;
                        let (rect, _response) =
                            ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());

                        let cell_id = cell.id;
                        let shared = self.shared.clone();

                        let callback = egui::PaintCallback {
                            rect,
                            callback: Arc::new(egui_glow::CallbackFn::new(
                                move |info, painter| {
                                    let gl = painter.gl();
                                    let state = shared.lock().unwrap();
                                    let Some(program) = state.programs.get(&cell_id) else {
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
                                                vp.width_px as f32,
                                                vp.height_px as f32,
                                                1.0,
                                            );
                                        }
                                        gl.bind_vertex_array(Some(vao));
                                        gl.draw_arrays(glow::TRIANGLES, 0, 3);
                                        gl.bind_vertex_array(None);
                                        gl.use_program(None);
                                    }
                                },
                            )),
                        };
                        ui.painter().add(callback);

                        // Height slider
                        ui.add(
                            egui::Slider::new(&mut cell.preview_height, 64.0..=1024.0)
                                .text("preview height")
                                .show_value(false),
                        );

                        ui.separator();
                    });
                }

                if ui.button("+ New Cell").clicked() {
                    add = true;
                }
            });
        });

        if let Some(id) = remove_id {
            self.cells.retain(|c| c.id != id);
            let gl = &self.gl;
            let mut state = self.shared.lock().unwrap();
            if let Some(prog) = state.programs.remove(&id) {
                unsafe { gl.delete_program(prog) };
            }
        }
        if add {
            self.add_cell();
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

// --- Entry point ---

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
