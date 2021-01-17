#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 2, binding = 0) uniform Sprite_size {
    vec2 size;
};

layout(set = 3, binding = 0) uniform Transforms {
    mat4 InstanceTransforms[];
};

void main() {
    v_Uv = Vertex_Uv;
    vec3 position = Vertex_Position * vec3(size, 1.0);
    gl_Position = ViewProj * InstanceTransforms[0] *vec4(position, 1.0);
}
