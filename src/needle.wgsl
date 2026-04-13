struct Params {
    l:    f32,
    d:    f32,
    seed: u64,
    n:    u64,
}

@group(0) @binding(0) var<uniform>            p:     Params;
@group(0) @binding(1) var<storage, read_write> total: atomic<u64>;

var<workgroup> tile: array<u64, 64>;

fn pcg(s: u32) -> u32 {
    let st = s * 747796405u + 2891336453u;
    let w  = ((st >> ((st >> 28u) + 4u)) ^ st) * 277803737u;
    return (w >> 22u) ^ w;
}

fn rf(s: u32) -> f32 {
    return f32(pcg(s)) / 4294967295.0;
}

@compute @workgroup_size(64)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id)  lid: vec3<u32>,
    @builtin(num_workgroups)       nwg: vec3<u32>,
) {
    let i: u64 = u64(gid.y) * u64(nwg.x) * u64(64) + u64(gid.x);

    var cross: u64 = 0;
    if i < p.n {
        let s0    = pcg(u32(p.seed + i * u64(3u)));
        let s1    = pcg(s0 + 1u);
        let y0    = rf(s0) * p.d;
        let angle = (rf(s1) * 2.0 - 1.0) * 3.14159265358979;
        let y1    = y0 + p.l * cos(angle);
        cross     = u64(y1 >= p.d || y1 < 0.0);
    }

    tile[lid.x] = cross;
    workgroupBarrier();

    var stride = 32u;
    loop {
        if stride == 0u { break; }
        if lid.x < stride {
            tile[lid.x] += tile[lid.x + stride];
        }
        workgroupBarrier();
        stride >>= 1u;
    }

    if lid.x == 0u {
        atomicAdd(&total, tile[0]);
    }
}
