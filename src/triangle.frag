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

    sampler2D specularGlossinessTexture;
    float specularFactor;
    float glossinessFactor;
    bool specularIsTexture;
    bool glossinessIsTexture;

    sampler2D normalTexture;
    bool hasNormalTexture;
};

uniform Material material;


void main()
{
    if (material.diffuseIsTexture)
        FragColor = texture(material.diffuseTexture, fs_in.texCoord);
    else
        FragColor = material.diffuseFactor;
}
