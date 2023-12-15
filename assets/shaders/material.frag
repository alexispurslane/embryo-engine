#version 430 core

out vec4 FragColor;

in VS_OUT {
    vec3 normal;
    vec2 texCoord;
    vec4 tangent;
} fs_in;

struct Material {
    sampler2D diffuseTexture;
    vec4 diffuseFactor;
    bool diffuseIsTexture;
};

uniform Material material;


void main()
{
    if (material.diffuseIsTexture)
        FragColor = texture(material.diffuseTexture, fs_in.texCoord);
    else
        FragColor = material.diffuseFactor;
}
