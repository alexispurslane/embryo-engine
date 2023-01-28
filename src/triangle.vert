#version 430 core

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aColor;
layout (location = 2) in vec2 aTexCoord;
layout (location = 3) in vec4 instPos;

uniform mat4 mvp;

out VS_OUT {
    vec3 color;
    vec2 texCoord;
} vs_out;

void main()
    {
        gl_Position = mvp * vec4(aPos, 1.0) + instPos;
        vs_out.color = aColor;
        vs_out.texCoord = aTexCoord;
    }