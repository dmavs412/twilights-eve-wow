// The Closed Zones' mist curtain (`crate::mist`). The colour is the scene fog's, raw gamma like
// every fogged surface. The alpha is opaque over the foot's band and fades to nothing at the top,
// and two layers of value noise drifting sideways along the curtain thin it to 0.75. The noise
// reads world coordinates, so neighbouring segments line up.

#import bevy_pbr::forward_io::VertexOutput

// rgb: the scene fog colour; w: seconds of drift.
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> fog: vec4<f32>;
// x: the opaque band in yards; y, z: the two layers' drift in yards per second.
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> shape: vec4<f32>;

// Noise cells per yard for the two layers.
const FAST_SCALE: f32 = 1.0 / 9.0;
const SLOW_SCALE: f32 = 1.0 / 23.0;

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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let above_foot = in.uv.x;
    let height = in.uv.y;
    let opaque = shape.x;
    var band = 1.0;
    if above_foot > opaque {
        band = clamp(1.0 - (above_foot - opaque) / max(height - opaque, 0.001), 0.0, 1.0);
    }
    // A segment runs along one world axis, so the sum of the two is its length coordinate, the
    // same at a shared corner.
    let along = in.world_position.x + in.world_position.z;
    let up = in.world_position.y;
    let t = fog.w;
    let fast = value_noise(vec2<f32>((along + shape.y * t) * FAST_SCALE, up * FAST_SCALE));
    let slow = value_noise(vec2<f32>((along - shape.z * t) * SLOW_SCALE, up * SLOW_SCALE));
    let thin = mix(0.75, 1.0, 0.5 * (fast + slow));
    return vec4<f32>(fog.rgb, band * thin);
}
