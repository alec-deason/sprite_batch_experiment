#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;

layout(location = 0) out vec2 v_Uv;
layout(location = 1) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform TextureAtlas_size {
    vec2 AtlasSize;
};
struct Rect {
    vec2 begin;
    vec2 end;
};
layout(set = 1, binding = 1) buffer TextureAtlas_textures {
    Rect[] Textures;
};

layout(set = 2, binding = 0) uniform Transforms {
    mat4 InstanceTransforms[200];
};
layout(set = 2, binding = 1) uniform Colors {
    vec4 InstanceColors[200];
};
layout(set = 2, binding = 2) uniform AtlasIndexes {
    uint InstanceAtlasIndexes[200];
};

void main() {
    Rect sprite_rect = Textures[InstanceAtlasIndexes[gl_InstanceIndex]];
    vec2 sprite_dimensions = sprite_rect.end - sprite_rect.begin;
    vec3 vertex_position = vec3(Vertex_Position.xy * sprite_dimensions, 0.0);
    vec2 atlas_positions[4] = vec2[](
        vec2(sprite_rect.begin.x, sprite_rect.end.y),
        sprite_rect.begin,
        vec2(sprite_rect.end.x, sprite_rect.begin.y),
        sprite_rect.end
    );
    v_Uv = (atlas_positions[gl_VertexIndex]) / AtlasSize;
    v_Color = InstanceColors[gl_InstanceIndex];
    gl_Position = ViewProj * InstanceTransforms[gl_InstanceIndex] * vec4(vertex_position, 1.0);
}
