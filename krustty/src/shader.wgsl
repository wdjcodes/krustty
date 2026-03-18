// We just pass the window width and height in pixels (e.g., 1920.0, 1080.0)
struct Globals {
    surface_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>, // Just 2D now!
    @location(1) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(2) surface_pos: vec2<f32>, // The pixel (X, Y) of the top-left of the cell
    @location(3) atlas_uv: vec4<f32>,
    @location(4) fg_color: vec4<f32>,
    @location(5) bg_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg: vec4<f32>,
    @location(2) bg: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // 1. Calculate the raw pixel position of this vertex
    let pixel_pos = model.position + instance.surface_pos;
    
    // 2. Convert pixel X to NDC X (-1.0 to 1.0)
    let ndc_x = (pixel_pos.x / globals.surface_size.x) * 2.0 - 1.0;
    
    // 3. Convert pixel Y to NDC Y (1.0 to -1.0)
    // Note: We subtract from 1.0 because window Y goes DOWN, but NDC Y goes UP
    let ndc_y = 1.0 - (pixel_pos.y / globals.surface_size.y) * 2.0;
    
    // 4. Output the final coordinate (Z is 0.0, W is 1.0)
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    
    // UV and color pass-through remains exactly the same
    let u = mix(instance.atlas_uv.x, instance.atlas_uv.z, model.tex_coords.x);
    let v = mix(instance.atlas_uv.y, instance.atlas_uv.w, model.tex_coords.y);
    out.uv = vec2<f32>(u, v);
    out.fg = instance.fg_color;
    out.bg = instance.bg_color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(t_diffuse, s_diffuse, in.uv).r;
    // let coverage = mix(0.0, 1.0, in.clip_position.x);
    return mix(in.bg, in.fg, coverage);
}