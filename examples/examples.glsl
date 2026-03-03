// === Plasma ===
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float t = iTime;
    float v = sin(uv.x * 10.0 + t) + sin(uv.y * 10.0 + t) + sin((uv.x + uv.y) * 10.0 + t);
    fragColor = vec4(0.5 + 0.5 * cos(v + vec3(0.0, 2.1, 4.2)), 1.0);
}

// === Spinning Rings ===
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float d = length(uv);
    float a = atan(uv.y, uv.x);
    float ring = sin(d * 30.0 - iTime * 3.0 + a * 5.0);
    vec3 col = mix(vec3(0.1, 0.0, 0.2), vec3(0.0, 0.8, 1.0), smoothstep(-0.2, 0.2, ring));
    col *= 1.0 - smoothstep(0.4, 0.5, d);
    fragColor = vec4(col, 1.0);
}

// === Voronoi Cells ===
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy * 5.0;
    float minDist = 1.0;
    vec2 minPoint;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            vec2 cell = floor(uv) + vec2(x, y);
            vec2 point = cell + 0.5 + 0.4 * sin(iTime + 6.2831 * fract(sin(dot(cell, vec2(127.1, 311.7))) * 43758.5453));
            float d = length(uv - point);
            if (d < minDist) { minDist = d; minPoint = cell; }
        }
    }
    vec3 col = 0.5 + 0.5 * cos(6.2831 * fract(sin(dot(minPoint, vec2(127.1, 311.7))) * 43758.5453) + vec3(0.0, 2.0, 4.0));
    col *= 1.0 - 0.5 * minDist;
    fragColor = vec4(col, 1.0);
}
