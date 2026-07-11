#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 tex_coord;

uniform mat4 transform;
uniform mat3 normal_transform;

out vec3 Normal;
out vec2 TexCoord;

void main() {
  gl_Position = transform * vec4(position, 1.0);
  Normal = normalize(normal_transform * normal);
  TexCoord = tex_coord;
}
