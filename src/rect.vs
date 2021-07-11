#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 texture_pos;
layout(location = 2) in uint texture_index;

layout(location = 0) out vec2 v_tex_pos;
layout(location = 1) out uint v_tex_index;

void main() {
    v_tex_pos=texture_pos;
    v_tex_index=texture_index;

    gl_Position = vec4(pos, 1.0, 1.0);
}