// The Closed Zones' mist curtain (`crate::mist`). The colour is the scene fog's, raw gamma like
// every fogged surface, desaturated and darkened at the foot and back to the pure fog colour higher
// up, so the top dissolves into the sky; a second noise lightens and darkens it in billows, and
// the scene's linear distance fog takes it far away. The alpha is nearly opaque over the foot's
// band, with faint billows, and fades to nothing at the top. A warped fractal of gradient noise,
// drifting sideways and upward, moves the fade's start up and down and thins the fade, so the top
// is ragged. The noise reads world coordinates, so neighbouring segments line up, and its lattice
// wraps every `NOISE_PERIOD_YD`, so the drift clock can wrap without a jump. Each piece of the mesh
// carries its layer number in the vertex colour's `x` (the backdrop 0, the two wisp sheets 1 and
// 2, the ground skirt 3): each layer reads its own noise and drifts at its own speed and way. Each
// piece fades in above its own foot under its own alpha ceiling, and the wisps and the skirt
// vanish close to the camera.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
}

// rgb: the scene fog colour; w: seconds of drift, wrapped at `DRIFT_WRAP_SECS`.
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fog: vec4<f32>;
// x: the opaque band in yards; y, z: the noise's sideways and upward drift in yards per second,
// before each layer's multiplier; w: the domain warp in yards.
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> shape: vec4<f32>;
// x: the foot's desaturation; y: its brightness; z: the yards its colour takes to reach the fog's;
// w: the yards each piece takes to fade in above its own foot.
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var<uniform> foot: vec4<f32>;
// x: the yards the noise moves the fade's start; y: the fade's alpha ceiling at the lowest noise;
// z: the yards from the camera within which the wisps and the skirt vanish; w: the fade factor,
// 1 but on a curtain fading out because its zone opened.
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var<uniform> rag: vec4<f32>;
// x, y: the scene's linear fog start and end in yards; z: `BILLOW_LIGHT`, the share the billows
// lighten or darken the colour by; w: `BAND_FLOOR`, the solid band's alpha at the lowest noise.
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var<uniform> scene: vec4<f32>;
// `LAYER_DRIFT`, each layer's multiplier of the sideways drift: layers 0 to 3 in the first
// vector, layer 4 in the second's x.
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var<uniform> drift: array<vec4<f32>, 2>;

// Noise cells per yard of the first octave.
const NOISE_SCALE: f32 = 1.0 / 23.0;
// Yards after which every octave repeats: 256 first-octave cells. Matches `NOISE_PERIOD_YD`.
const NOISE_PERIOD_YD: f32 = 5888.0;
// Lattice cells of the first octave in one period; octave k has 2^k times as many.
const NOISE_PERIOD_CELLS: i32 = 256;
// The last entry of `LAYER_DRIFT`, which layers past the end take.
const LAST_LAYER: u32 = 4u;

// An integer hash of a lattice point, in [0, 1].
fn hash(p: vec2<i32>) -> f32 {
    var h = (bitcast<u32>(p.x) * 1597334673u) ^ (bitcast<u32>(p.y) * 3812015801u);
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = h ^ (h >> 13u);
    return f32(h) / 4294967295.0;
}

// The gradient at lattice point `i`, wrapped onto `period` cells both ways, dotted with the
// offset `f` from that point.
fn corner(i: vec2<i32>, period: i32, f: vec2<f32>) -> f32 {
    let w = ((i % period) + vec2<i32>(period)) % period;
    let angle = hash(w) * 6.2831853;
    return dot(vec2<f32>(cos(angle), sin(angle)), f);
}

// 2D gradient noise at `q` in cells, its lattice repeating every `period` cells, in [0, 1].
fn gradient_noise(q: vec2<f32>, period: i32) -> f32 {
    let cell = floor(q);
    let i = vec2<i32>(cell);
    let f = q - cell;
    // The quintic fade, so the slope is smooth across cells too.
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = corner(i, period, f);
    let b = corner(i + vec2<i32>(1, 0), period, f - vec2<f32>(1.0, 0.0));
    let c = corner(i + vec2<i32>(0, 1), period, f - vec2<f32>(0.0, 1.0));
    let d = corner(i + vec2<i32>(1, 1), period, f - vec2<f32>(1.0, 1.0));
    let v = mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
    // Unit gradients keep `v` within ±√2/2.
    return clamp(v * 0.70710678 + 0.5, 0.0, 1.0);
}

// `octaves` octaves of gradient noise at a point in yards, in [0, 1]. Octave k has cells of
// 23 / 2^k yd and weight 0.5^(k + 1), normalised; every octave repeats every `NOISE_PERIOD_YD`.
fn fractal(p: vec2<f32>, octaves: i32) -> f32 {
    // Into the first period, which keeps the lattice coordinates small; the noise is the same.
    let q = (p - NOISE_PERIOD_YD * floor(p / NOISE_PERIOD_YD)) * NOISE_SCALE;
    var sum = 0.0;
    var total = 0.0;
    var weight = 0.5;
    var scale = 1.0;
    var period = NOISE_PERIOD_CELLS;
    for (var k = 0; k < octaves; k = k + 1) {
        sum = sum + weight * gradient_noise(q * scale, period);
        total = total + weight;
        weight = weight * 0.5;
        scale = scale * 2.0;
        period = period * 2;
    }
    return sum / total;
}

// Layer `layer`'s multiplier of the sideways drift; layers past the table take its last entry.
fn layer_drift(layer: u32) -> f32 {
    let l = min(layer, LAST_LAYER);
    return drift[l / 4u][l % 4u];
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Yards above the terrain vertex in this column, and the sheet top's.
    let above_ground = in.uv.x;
    let top_above_ground = in.uv.y;
    // The piece's alpha ceiling, and yards above the piece's own foot.
    let cap = in.uv_b.x;
    let ramp = in.uv_b.y;
    // The piece's layer number; a mesh with no colours is all layer 0.
    var layer = 0u;
#ifdef VERTEX_COLORS
    layer = u32(max(round(in.color.x), 0.0));
#endif
    // A segment runs along one world axis, so the sum of the two is its length coordinate, the
    // same at a shared corner.
    let along = in.world_position.x + in.world_position.z;
    let up = in.world_position.y;
    let t = fog.w;
    let m = layer_drift(layer);
    // Each layer's own noise, far from the others', drifting its own way.
    let p = vec2<f32>(along, up) + f32(layer) * vec2<f32>(137.0, 71.0)
        + vec2<f32>(-shape.y * m, -shape.z * abs(m)) * t;
    // The domain warp: two more samples, far from `p` and from each other, push the point.
    let warp = vec2<f32>(fractal(p + vec2<f32>(57.0, 13.0), 2), fractal(p + vec2<f32>(-31.0, 89.0), 2));
    let pw = p + (warp - 0.5) * 2.0 * shape.w;
    let n = fractal(pw, 4);
    let billow = fractal(pw + vec2<f32>(311.0, 173.0), 2);
    let start = shape.x + (n - 0.5) * 2.0 * rag.x;
    var band = mix(scene.w, 1.0, n);
    if above_ground > start {
        band = clamp(1.0 - (above_ground - start) / max(top_above_ground - start, 0.001), 0.0, 1.0);
        band = band * mix(rag.y, 1.0, n);
    }
    let luma = dot(fog.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let dark = mix(fog.rgb, vec3<f32>(luma), foot.x) * foot.y;
    var colour = mix(dark, fog.rgb, smoothstep(0.0, foot.z, above_ground));
    colour = colour * (1.0 + (billow - 0.5) * 2.0 * scene.z);
    // The scene's linear fog on planar eye-Z, as the terrain takes it.
    let eye_z = -(view.view_from_world * vec4<f32>(in.world_position.xyz, 1.0)).z;
    let factor = clamp((scene.y - eye_z) / max(scene.y - scene.x, 0.001), 0.0, 1.0);
    colour = mix(fog.rgb, colour, factor);
    var alpha = band * cap * smoothstep(0.0, foot.w, ramp);
    // Only the backdrop has the full ceiling; the wisps and the skirt fade out at the camera. The
    // margin under 1 keeps an interpolated 0.99999 on the backdrop from counting.
    if cap < 0.99 {
        let to_camera = distance(in.world_position.xyz, view.world_position.xyz);
        alpha = alpha * smoothstep(0.5, rag.z, to_camera);
    }
    return vec4<f32>(colour, alpha * rag.w);
}
