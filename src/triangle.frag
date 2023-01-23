#version 450 core

out vec4 Color;

uniform vec3 GlobalColor;

void main()
    {
        Color = vec4(GlobalColor, 1.0f);
    }