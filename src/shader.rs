pub const VERTEX_SHADER: &str = "#version 300 es\n\
precision highp float;\n\
void main() {\n\
    float x = float((gl_VertexID & 1) << 2) - 1.0;\n\
    float y = float((gl_VertexID & 2) << 1) - 1.0;\n\
    gl_Position = vec4(x, y, 0.0, 1.0);\n\
}\n";

pub const FRAG_PREAMBLE: &str = "#version 300 es\n\
precision highp float;\n\
uniform float iTime;\n\
uniform vec3 iResolution;\n\
uniform vec2 iOffset;\n\
out vec4 fragColor;\n";

pub const FRAG_POSTAMBLE: &str =
    "\nvoid main() { mainImage(fragColor, gl_FragCoord.xy - iOffset); }\n";

pub fn default_source(id: usize) -> String {
    format!(
        "\
# shader {id} 1280x720
void mainImage(out vec4 fragColor, in vec2 fragCoord) {{
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}}"
    )
}

pub fn parse_header(source: &str) -> (f32, f32) {
    let first_line = source.lines().next().unwrap_or("");
    if let Some(rest) = first_line.strip_prefix("# ") {
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

pub fn shader_code(source: &str) -> &str {
    source.find('\n').map(|i| &source[i + 1..]).unwrap_or("")
}

pub fn build_frag(user: &str) -> String {
    if user.contains("void main(") {
        format!("{FRAG_PREAMBLE}\n{user}")
    } else {
        format!("{FRAG_PREAMBLE}\n{user}\n{FRAG_POSTAMBLE}")
    }
}
