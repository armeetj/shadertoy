use eframe::egui;
use crate::theme::*;

pub fn highlight(text: &str, font_id: &egui::FontId) -> egui::text::LayoutJob {
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
