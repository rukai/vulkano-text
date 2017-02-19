#version 450

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_450pack : enable

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_position;
layout(location = 2) in vec4 color;
layout(location = 0) out vec2 v_tex_position;
layout(location = 1) out vec4 v_color;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    v_tex_position = tex_position;
    v_color = color;
}
