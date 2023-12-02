#version 430 core

out vec4 FragColor;

in VS_OUT {
    vec3 normal;
    vec2 texCoord;
} fs_in;

struct Material {
    sampler2D baseColorTexture;
    vec4 baseColorFactor;
    bool hasTexture;
};

uniform Material material;


void main()
{
    if (material.hasTexture)
        FragColor = texture(material.baseColorTexture, fs_in.texCoord);
    else
        FragColor = material.baseColorFactor;
}
