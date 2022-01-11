[[stage(vertex)]]
fn vs_main(
    [[location(0)]] point: vec2<f32>,
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(point, 0.0, 1.0);
}

[[stage(fragment)]]
fn fs_main_black() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
[[stage(fragment)]]
fn fs_main_red() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
[[stage(fragment)]]
fn fs_main_green() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
[[stage(fragment)]]
fn fs_main_blue() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.0, 0.0, 1.0, 1.0);
}