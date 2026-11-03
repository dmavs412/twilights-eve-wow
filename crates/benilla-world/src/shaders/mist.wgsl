// The Closed Zones' mist curtain (`crate::mist`). The colour is the scene fog's, raw gamma like
// every fogged surface, desaturated and darkened at the foot and back to the pure fog colour higher
// up, so the top dissolves into the sky. The alpha is opaque over the foot's band and fades to
// nothing at the top. One warped fractal noise, drifting sideways and upward, moves the fade's
// start up and down and thins the fade, so the top is ragged. The noise reads world coordinates,
// so neighbouring segments line up. Each piece of the mesh (the backdrop, the two wisp sheets and
// the ground skirt) fades in above its own foot under its own alpha ceiling.

#import bevy_pbr::forward_io::VertexOutput

// rgb: the scene fog colour; w: seconds of drift.
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fog: vec4<f32>;
// x: the opaque band in yards; y, z: the noise's sideways and upward drift in yards per second;
// w: the domain warp in yards.
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> shape: vec4<f32>;
// x: the foot's desaturation; y: its brightness; z: the yards its colour takes to reach the fog's;
// w: the yards each piece takes to fade in above its own foot.
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> foot: vec4<f32>;
// x: the yards the noise moves the fade's start; y: the fade's alpha ceiling at the lowest noise.
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var<uniform> rag: vec4<f32>;

// Noise cells per yard of the first octave.
const NOISE_SCALE: f32 = 1.0 / 23.0;

// An integer hash of a lattice point, in [0, 1].
fn hash(p: vec2<i32>) -> f32 {
    var h = (bitcast<u32>(p.x) * 1597334673u) ^ (bitcast<u32>(p.y) * 3812015801u);
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h) / 4294967295.0;
}

// Smooth value noise in [0, 1].
fn value_noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let i = vec2<i32>(cell);
    let f = p - cell;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<i32>(1, 0));
    let c = hash(i + vec2<i32>(0, 1));
    let d = hash(i + vec2<i32>(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Three octaves of value noise at a point in yards, in [0, 1].
fn fractal(p: vec2<f32>) -> f32 {
    let q = p * NOISE_SCALE;
    let sum = 0.5 * value_noise(q) + 0.25 * value_noise(q * 2.0) + 0.125 * value_noise(q * 4.0);
    return sum / 0.875;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Yards above the terrain vertex in this column, and the sheet top's.
    let above_ground = in.uv.x;
    let top_above_ground = in.uv.y;
    // The piece's alpha ceiling, and yards above the piece's own foot.
    let cap = in.uv_b.x;
    let ramp = in.uv_b.y;
    // A segment runs along one world axis, so the sum of the two is its length coordinate, the
    // same at a shared corner.
    let along = in.world_position.x + in.world_position.z;
    let up = in.world_position.y;
    let t = fog.w;
    let p = vec2<f32>(along - shape.y * t, up - shape.z * t);
    // The domain warp: two more samples, far from `p` and from each other, push the point.
    let warp = vec2<f32>(fractal(p + vec2<f32>(57.0, 13.0)), fractal(p + vec2<f32>(-31.0, 89.0)));
    let n = fractal(p + (warp - 0.5) * 2.0 * shape.w);
    let start = shape.x + (n - 0.5) * 2.0 * rag.x;
    var band = 1.0;
    if above_ground > start {
        band = clamp(1.0 - (above_ground - start) / max(top_above_ground - start, 0.001), 0.0, 1.0);
        band = band * mix(rag.y, 1.0, n);
    }
    let luma = dot(fog.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dark = mix(fog.rgb, vec3<f32>(luma), foot.x) * foot.y;
    let colour = mix(dark, fog.rgb, smoothstep(0.0, foot.z, above_ground));
    return vec4<f32>(colour, band * cap * smoothstep(0.0, foot.w, ramp));
}
