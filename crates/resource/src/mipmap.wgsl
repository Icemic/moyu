struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let tex_coords = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));

    var output: VertexOutput;
    output.position = vec4<f32>(tex_coords * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return output;
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let source_size = textureDimensions(source_texture);
    let destination_size = max(source_size / 2u, vec2<u32>(1u));
    let destination_position = vec2<u32>(input.position.xy);
    let source_start = vec2<f32>(destination_position) * vec2<f32>(source_size)
        / vec2<f32>(destination_size);
    let source_end = vec2<f32>(destination_position + 1u) * vec2<f32>(source_size)
        / vec2<f32>(destination_size);
    let first_source_texel = vec2<i32>(floor(source_start));

    var color = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var y = 0; y < 3; y++) {
        let source_y = first_source_texel.y + y;
        let weight_y = max(
            0.0,
            min(source_end.y, f32(source_y + 1)) - max(source_start.y, f32(source_y)),
        );

        for (var x = 0; x < 3; x++) {
            let source_x = first_source_texel.x + x;
            let weight_x = max(
                0.0,
                min(source_end.x, f32(source_x + 1)) - max(source_start.x, f32(source_x)),
            );
            let weight = weight_x * weight_y;

            if weight > 0.0 {
                color += textureLoad(source_texture, vec2<i32>(source_x, source_y), 0) * weight;
                total_weight += weight;
            }
        }
    }

    return color / total_weight;
}
