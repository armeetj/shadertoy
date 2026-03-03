use glow::HasContext;

pub fn compile_shader(gl: &glow::Context, ty: u32, src: &str) -> Result<glow::Shader, String> {
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

pub fn link_program(
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
