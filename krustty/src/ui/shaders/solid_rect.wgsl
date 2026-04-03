struct Globals {
    surface_size: vec2<f32>,
    cell_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct RectInstance {
    @location(1) screen_pos: vec2<f32>, // Top-left pixel
    @location(2) size: vec2<f32>,       // Width and height in pixels
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput, instance: RectInstance) -> VertexOutput {
    var out: VertexOutput;
    
    // Scale the 1x1 unit quad by the instance size, then translate
    let pixel_pos = (model.position * instance.size) + instance.screen_pos;
    
    // Convert to NDC (same as your text shader)
    let ndc_x = (pixel_pos.x / globals.surface_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_pos.y / globals.surface_size.y) * 2.0;
    
    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}