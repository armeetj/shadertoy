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

pub const DEFAULT_SHADER_0: &str = r#"# fractal pyramid 1280x720
vec3 palette(float d){
    return mix(vec3(0.2,0.7,0.9),vec3(1.,0.,1.),d);
}

vec2 rotate(vec2 p,float a){
    float c = cos(a);
    float s = sin(a);
    return p*mat2(c,s,-s,c);
}

float map(vec3 p){
    for( int i = 0; i<8; ++i){
        float t = iTime*0.2;
        p.xz =rotate(p.xz,t);
        p.xy =rotate(p.xy,t*1.89);
        p.xz = abs(p.xz);
        p.xz-=.5;
    }
    return dot(sign(p),p)/5.;
}

vec4 rm (vec3 ro, vec3 rd){
    float t = 0.;
    vec3 col = vec3(0.);
    float d;
    for(float i =0.; i<64.; i++){
        vec3 p = ro + rd*t;
        d = map(p)*.5;
        if(d<0.02){
            break;
        }
        if(d>100.){
            break;
        }
        col+=palette(length(p)*.1)/(400.*(d));
        t+=d;
    }
    return vec4(col,1./(d*100.));
}

void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
    vec2 uv = (fragCoord-(iResolution.xy/2.))/iResolution.x;
    vec3 ro = vec3(0.,0.,-50.);
    ro.xz = rotate(ro.xz,iTime);
    vec3 cf = normalize(-ro);
    vec3 cs = normalize(cross(cf,vec3(0.,1.,0.)));
    vec3 cu = normalize(cross(cf,cs));

    vec3 uuv = ro+cf*3. + uv.x*cs + uv.y*cu;

    vec3 rd = normalize(uuv-ro);

    vec4 col = rm(ro,rd);

    fragColor = col;
}"#;

pub const DEFAULT_SHADER_1: &str = r#"# fbm noise 1280x720
mat2 m = mat2(0.80, 0.60,-0.60, 0.80);

float noise(vec2 p) {
    return sin(p.x) * sin(p.y);
}

float fbm4(vec2 p) {
    float f = 0.0;
    float g = 1.0;
    for(int i =0;i<4;i++){
        g/=2.0;
        f += g * noise(p);
        p = m * p * 2.05;
    }
    return f / 0.9375;
}

float fbm6(vec2 p) {
    float f = 0.0;
    float g = 1.0;
    for(int i =0;i<6;i++){
        g/=2.0;
        f += g * (0.5 + 0.3 * noise(p));
        p = m * p * 2.02;
    }
    return f / 0.96875;
}

vec2 fbm4_2(vec2 p) {
    return vec2(fbm4(p), fbm4(p + vec2(7.8, 8.0)));
}

vec2 fbm6_2(vec2 p) {
    return vec2(fbm6(p + vec2(16.8, 20.0)),fbm6(p + vec2(11.5, 0.0)));
}

float func(vec2 q, inout vec4 ron, float time) {
    q += 0.03 * sin(vec2(0.27, 0.23) * time + length(q) * vec2(4.1, 4.3));
    vec2 o = fbm4_2(0.9 * q);
    o += 0.04 * sin(vec2(0.12, 0.14) * time + length(o));
    vec2 n = fbm6_2(3.0 * o);
    ron = vec4(o, n);
    float f = 0.5 + 0.5 * fbm4(1.8 * q + 6.0 * n);
    return mix(f, f * f * f * 3.5, f * abs(n.x));
}

void mainImage( out vec4 fragColor, in vec2 fragCoord ) {
    vec2 p = (2.0 * fragCoord - iResolution.xy) / iResolution.y;
    p *= 2.0;

    float e = 2.0 / iResolution.y;

    vec4 on;
    float f = func(p, on, iTime*10.0);
    vec3 colo;
    colo = mix(vec3(0.2, 0.1, 0.4), vec3(0.3, 0.05, 0.05), f);
    colo = mix(colo, vec3(0.3, 0.9, 0.9), dot(on.zw, on.zw));
    colo = mix(colo, vec3(0.4, 0.3, 0.9), 0.2 + 0.5 * on.y * on.y);
    colo = mix(colo, vec3(0.9, 0.9, 0.4), 0.5 * smoothstep(1.2, 1.3, abs(on.z) + abs(on.w)));
    colo = clamp(colo * f * 2.0, 0.0, 1.0);

    vec4 kk;
    vec3 nor = normalize(vec3(
        func(p + vec2(e, 0.0), kk, iTime*10.0) - f,
        2.0 * e,
        func(p + vec2(0.0, e), kk, iTime*10.0) - f
    ));
    vec3 lig = normalize(vec3(0.9, 0.2, -0.4));
    float dif = clamp(0.3 + 0.7 * dot(nor, lig), 0.0, 1.0);
    vec3 lin = vec3(0.70, 0.90, 0.95) * (nor.y * 0.5 + 0.5) + vec3(0.15, 0.10, 0.05) * dif;

    colo *= 1.0 * lin;
    colo = 2.5 * colo * colo;

    fragColor = vec4(colo, 1.0);
}"#;

pub const DEFAULT_SHADER_2: &str = r#"# simple uv 1280x720
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}"#;

pub fn default_source(id: usize) -> String {
    match id {
        0 => DEFAULT_SHADER_0.to_string(),
        1 => DEFAULT_SHADER_1.to_string(),
        2 => DEFAULT_SHADER_2.to_string(),
        _ => format!(
            "\
# shader {id} 1280x720
void mainImage(out vec4 fragColor, in vec2 fragCoord) {{
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);
}}"
        ),
    }
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
