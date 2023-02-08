#version 430 core

out vec4 FragColor;

in VS_OUT {
    vec3 normal;
    vec2 texCoord;
} fs_in;

uniform sampler2D texture0;
uniform sampler2D texture1;

void main()
{
    FragColor = mix(texture(texture1, fs_in.texCoord), texture(texture0, fs_in.texCoord), 0.5);
}