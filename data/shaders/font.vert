#version 430 core

layout (location = 0) in vec2 aPosition;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoords;

uniform mat4 model_matrix;
uniform mat4 projection_matrix;

void main() {
    gl_Position = projection_matrix * model_matrix * vec4(aPosition, 0.0, 1.0);
    TexCoords = aTexCoord;
}
