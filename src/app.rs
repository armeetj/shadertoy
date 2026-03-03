use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use glow::HasContext;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use web_time::Instant;

use crate::gl_utils::{compile_shader, link_program};
use crate::highlight::highlight;
use crate::shader::*;
use crate::theme::*;

pub struct ShaderState {
    pub programs: HashMap<usize, glow::Program>,
    pub vao: glow::VertexArray,
    pub time: f32,
}

pub struct Cell {
    pub id: usize,
    pub source: String,
    pub error: Option<String>,
    pub prev_code: String,
}

/// Cursor state for the focused cell, recorded at end of each frame.
struct FocusInfo {
    cell_id: usize,
    cursor_char: usize,
    on_first_line: bool,
    on_last_line: bool,
}

/// Pending focus change to apply before next TextEdit render.
struct FocusTarget {
    cell_id: usize,
    cursor_char: usize,
}

pub struct App {
    cells: Vec<Cell>,
    next_id: usize,
    gl: Arc<glow::Context>,
    vert: glow::Shader,
    shared: Arc<Mutex<ShaderState>>,
    t0: Instant,
    focus_info: Option<FocusInfo>,
    focus_target: Option<FocusTarget>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// True if the character at `char_idx` is on the first line of `source`.
fn cursor_on_first_line(source: &str, char_idx: usize) -> bool {
    match source.chars().position(|c| c == '\n') {
        Some(nl) => char_idx <= nl,
        None => true,
    }
}

/// True if the character at `char_idx` is on the last line of `source`.
fn cursor_on_last_line(source: &str, char_idx: usize) -> bool {
    match source.chars().enumerate().filter(|(_, c)| *c == '\n').last() {
        Some((nl, _)) => char_idx > nl,
        None => true,
    }
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Compute the egui Id for a cell's TextEdit.
fn text_edit_id(cell_id: usize) -> egui::Id {
    egui::Id::new(("cell_editor", cell_id))
}

/// Number of lines generated before user code in `build_frag`.
/// FRAG_PREAMBLE ends with `\n`, then `format!` adds another `\n` separator.
fn preamble_line_count() -> usize {
    FRAG_PREAMBLE.chars().filter(|&c| c == '\n').count() + 1 // +1 for the \n in format!
}

/// Parse a GLSL error log into `(galley_row, message)` pairs.
/// Returns `(inline_errors, fallback_messages)`.
fn parse_glsl_errors(error_text: &str) -> (Vec<(usize, String)>, Vec<String>) {
    let offset = preamble_line_count(); // glsl_line - offset = galley_row
    let mut inline: Vec<(usize, String)> = Vec::new();
    let mut fallback: Vec<String> = Vec::new();

    for line in error_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try pattern: ERROR: <shader>:<line>: <message>  (or without ERROR: prefix)
        let work = line.strip_prefix("ERROR:").unwrap_or(line).trim();

        let mut found = false;
        if let Some(colon1) = work.find(':') {
            let before = &work[..colon1];
            if before.chars().all(|c| c.is_ascii_digit()) {
                let rest = &work[colon1 + 1..];
                if let Some(colon2) = rest.find(':') {
                    let line_str = rest[..colon2].trim();
                    if let Ok(glsl_line) = line_str.parse::<usize>() {
                        let msg = rest[colon2 + 1..].trim().to_string();
                        if !msg.is_empty() {
                            let row = glsl_line.saturating_sub(offset);
                            inline.push((row, msg));
                            found = true;
                        }
                    }
                }
            }
        }

        if !found {
            fallback.push(line.to_string());
        }
    }

    // Deduplicate: keep first error per row
    inline.sort_by_key(|(row, _)| *row);
    inline.dedup_by_key(|(row, _)| *row);

    (inline, fallback)
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            focus_info: None,
            focus_target: None,
        };
        app.cells.push(Cell {
            id: 0,
            source: default_source(0),
            error: None,
            prev_code: String::new(),
        });
        app.next_id = 1;
        app
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

    /// Split any cell that contains `\n# ` (a new header mid-cell).
    fn split_cells(&mut self) {
        let focus_cell_id = self.focus_info.as_ref().map(|fi| fi.cell_id);
        let focus_cursor = self.focus_info.as_ref().map(|fi| fi.cursor_char).unwrap_or(0);

        let mut i = 0;
        while i < self.cells.len() {
            if let Some(split_byte) = self.cells[i].source.find("\n# ") {
                let before = self.cells[i].source[..split_byte].to_string();
                let after = self.cells[i].source[split_byte + 1..].to_string(); // skip the \n
                let old_id = self.cells[i].id;

                self.cells[i].source = before;
                self.cells[i].prev_code.clear(); // force recompile

                let new_id = self.next_id;
                self.next_id += 1;
                self.cells.insert(
                    i + 1,
                    Cell {
                        id: new_id,
                        source: after,
                        error: None,
                        prev_code: String::new(),
                    },
                );

                // If the focused cell was the one we just split, and cursor was past
                // the split point, move focus into the new cell.
                if focus_cell_id == Some(old_id) {
                    let split_chars = char_count(&self.cells[i].source); // chars in "before"
                    if focus_cursor > split_chars {
                        self.focus_target = Some(FocusTarget {
                            cell_id: new_id,
                            cursor_char: focus_cursor.saturating_sub(split_chars + 1),
                        });
                    }
                }

                i += 1; // advance to the new cell (it might contain further splits)
            } else {
                i += 1;
            }
        }
    }

    /// Remove cells whose source is empty (keep at least one).
    fn remove_empty_cells(&mut self) {
        if self.cells.len() <= 1 {
            return;
        }
        let gl = &self.gl;
        let shared = &self.shared;
        self.cells.retain(|c| {
            let keep = !c.source.trim().is_empty();
            if !keep {
                let mut st = shared.lock().unwrap();
                if let Some(prog) = st.programs.remove(&c.id) {
                    unsafe { gl.delete_program(prog) };
                }
            }
            keep
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = self.t0.elapsed().as_secs_f32();
        self.shared.lock().unwrap().time = time;

        // ---------------------------------------------------------------
        // 1. Input interception: Tab → spaces
        // ---------------------------------------------------------------
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

        // ---------------------------------------------------------------
        // 2. Cross-cell navigation interception
        //    Uses cursor state from the *previous* frame.
        //    If the cursor was already at a boundary and the user presses
        //    the arrow key again, we consume the event and schedule a
        //    focus transition.
        // ---------------------------------------------------------------
        if let Some(ref fi) = self.focus_info {
            let focus_idx = self.cells.iter().position(|c| c.id == fi.cell_id);

            if let Some(focus_idx) = focus_idx {
                let up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));

                // Up arrow on first line → jump to end of previous cell
                if up && fi.on_first_line && focus_idx > 0 {
                    ctx.input_mut(|i| {
                        i.events.retain(|e| {
                            !matches!(
                                e,
                                egui::Event::Key {
                                    key: egui::Key::ArrowUp,
                                    pressed: true,
                                    ..
                                }
                            )
                        });
                    });
                    let prev = &self.cells[focus_idx - 1];
                    self.focus_target = Some(FocusTarget {
                        cell_id: prev.id,
                        cursor_char: char_count(&prev.source),
                    });
                }

                // Down arrow on last line → jump to start of next cell
                if down && fi.on_last_line && focus_idx + 1 < self.cells.len() {
                    ctx.input_mut(|i| {
                        i.events.retain(|e| {
                            !matches!(
                                e,
                                egui::Event::Key {
                                    key: egui::Key::ArrowDown,
                                    pressed: true,
                                    ..
                                }
                            )
                        });
                    });
                    let next = &self.cells[focus_idx + 1];
                    self.focus_target = Some(FocusTarget {
                        cell_id: next.id,
                        cursor_char: 0,
                    });
                }

                // Backspace at position 0 → navigate to end of previous cell
                let backspace = ctx.input(|i| i.key_pressed(egui::Key::Backspace));
                if backspace && fi.cursor_char == 0 && focus_idx > 0 {
                    ctx.input_mut(|i| {
                        i.events.retain(|e| {
                            !matches!(
                                e,
                                egui::Event::Key {
                                    key: egui::Key::Backspace,
                                    pressed: true,
                                    ..
                                }
                            )
                        });
                    });
                    let prev = &self.cells[focus_idx - 1];
                    self.focus_target = Some(FocusTarget {
                        cell_id: prev.id,
                        cursor_char: char_count(&prev.source),
                    });
                }
            }
        }

        // ---------------------------------------------------------------
        // 3. Auto-split cells (detects \n# mid-cell and splits)
        // ---------------------------------------------------------------
        self.split_cells();

        // ---------------------------------------------------------------
        // 4. Remove empty cells
        // ---------------------------------------------------------------
        self.remove_empty_cells();

        // ---------------------------------------------------------------
        // 5. Compile shaders
        // ---------------------------------------------------------------
        for i in 0..self.cells.len() {
            self.compile_cell(i);
        }

        // ---------------------------------------------------------------
        // 6. Apply pending focus target (before UI so TextEdits pick it up)
        // ---------------------------------------------------------------
        if let Some(ft) = self.focus_target.take() {
            let te_id = text_edit_id(ft.cell_id);
            ctx.memory_mut(|mem| mem.request_focus(te_id));
            let mut state =
                egui::widgets::text_edit::TextEditState::load(ctx, te_id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(ft.cursor_char))));
            state.store(ctx, te_id);
        }

        // ---------------------------------------------------------------
        // 7. Show UI
        // ---------------------------------------------------------------
        let mut new_focus_info: Option<FocusInfo> = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(16.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let avail_w = ui.available_width();

                    for cell in &mut self.cells {
                        let cell_id = cell.id;
                        ui.push_id(cell_id, |ui| {
                            let font_id = egui::FontId::new(14.0, egui::FontFamily::Monospace);
                            let font_id_clone = font_id.clone();

                            let mut layouter =
                                |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                                    let job = highlight(text, &font_id_clone);
                                    ui.fonts(|f| f.layout_job(job))
                                };

                            let te_id = text_edit_id(cell_id);
                            let output = egui::TextEdit::multiline(&mut cell.source)
                                .id(te_id)
                                .font(egui::FontId::new(14.0, egui::FontFamily::Monospace))
                                .desired_width(avail_w)
                                .desired_rows(4)
                                .frame(false)
                                .lock_focus(true)
                                .layouter(&mut layouter)
                                .show(ui);

                            // Track cursor state for next frame's navigation logic.
                            if output.response.has_focus() {
                                if let Some(cursor_range) = output.cursor_range {
                                    let ch = cursor_range.primary.ccursor.index;
                                    new_focus_info = Some(FocusInfo {
                                        cell_id,
                                        cursor_char: ch,
                                        on_first_line: cursor_on_first_line(&cell.source, ch),
                                        on_last_line: cursor_on_last_line(&cell.source, ch),
                                    });
                                }
                            }

                            // ── Inline error diagnostics ──────────────
                            if let Some(err) = &cell.error {
                                let galley_pos = output.galley_pos;
                                let (inline_errs, fallback_errs) = parse_glsl_errors(err);
                                let err_font =
                                    egui::FontId::new(13.0, egui::FontFamily::Monospace);

                                for (row_idx, msg) in &inline_errs {
                                    let row_idx = *row_idx;
                                    if row_idx >= output.galley.rows.len() {
                                        continue;
                                    }
                                    let row = &output.galley.rows[row_idx];
                                    let row_rect =
                                        row.rect.translate(galley_pos.to_vec2());

                                    // Right edge of the actual text content
                                    let content_right = galley_pos.x
                                        + row
                                            .glyphs
                                            .last()
                                            .map(|g| g.pos.x + g.advance_width)
                                            .unwrap_or(0.0);

                                    // ■ error: message
                                    let mut job = egui::text::LayoutJob::default();
                                    job.append(
                                        "  ■ error: ",
                                        0.0,
                                        egui::TextFormat {
                                            font_id: err_font.clone(),
                                            color: RED,
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        msg,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: err_font.clone(),
                                            color: RED,
                                            ..Default::default()
                                        },
                                    );

                                    let err_galley =
                                        ui.fonts(|f| f.layout_job(job));

                                    let x = content_right;
                                    let y = row_rect.center().y
                                        - err_galley.rect.height() / 2.0;

                                    ui.painter().galley(
                                        egui::pos2(x, y),
                                        err_galley,
                                        RED,
                                    );
                                }

                                // Fallback: errors without line numbers
                                for msg in &fallback_errs {
                                    ui.label(
                                        egui::RichText::new(format!("■ error: {msg}"))
                                            .font(font_id.clone())
                                            .color(RED),
                                    );
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
                });
            });

        self.focus_info = new_focus_info;

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
