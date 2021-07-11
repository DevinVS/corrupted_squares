#version 450
layout(location = 0) in vec2 v_tex_pos;
layout(location = 1) in flat uint v_tex_index;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D textures[3];

void main() {
    f_color = texture(textures[v_tex_index], v_tex_pos);
}