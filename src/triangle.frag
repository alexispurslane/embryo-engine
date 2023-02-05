#version 430 core

out vec4 FragColor;

in VS_OUT {
    vec3 normal;
    vec2 texCoord;
} fs_in;

uniform sampler2D texture1;

void main()
{
    FragColor = texture(texture1, fs_in.texCoord) + vec4(1.0, 1.0, 1.0, 1.0);
}