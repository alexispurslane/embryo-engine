#version 430 core

#define RGB_TO_LUM vec3(0.2125, 0.7154, 0.0721)

layout (location = 0) out vec4 FragColor;
layout (location = 1) out vec4 BrightColor;

in VS_OUT {
    vec4 position;
    vec3 normal;
    vec2 texCoord;
    vec4 tangent;
} fs_in;

uniform sampler2D diffuseTexture;
uniform sampler2D specularTexture;
uniform vec4 diffuseFactor;
uniform bool diffuseIsTexture;
uniform vec3 specularFactor;
uniform bool specularIsTexture;
uniform float shininess;
uniform vec2 bloomThreshold = vec2(0.0, 1.2);

const uint Ambient = 0;
const uint Directional = 1;
const uint Point = 2;
const uint Spot = 3;

struct Light {
    vec3 position; // used in point and spot lights
    uint type;
    vec3 direction; // used in directional and spot lights
    float constantAttenuation; // used in point and spot lights
    vec3 ambient;
    float linearAttenuation; // used in point and spot lights
    vec3 color;
    float quadraticAttenuation; // used in point and spot lights
    float spotCutOff; // used in spot lights
    float spotExponent; // used in spot lights
};

layout (binding = 0, std140) uniform Lights {
    Light lights[32];
};

uniform uint lightmask;
uniform vec3 cameraDirection;

vec3 scatteredLight = vec3(0.0); // ambient and diffuse, color is a mix of object and light
vec3 reflectedLight = vec3(0.0); // specular, color is based on light alone

void directionalLight(
    Light light,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 halfVector = normalize(-light.direction + cameraDirection);
    diffuse = max(0.0, dot(fs_in.normal, light.direction));
    specular = max(0.0, dot(fs_in.normal, halfVector));
    attenuation = 1.0;
}

void pointLight(
    Light light,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 lightDirection = light.position - fs_in.position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;
    vec3 halfVector = normalize(lightDirection + cameraDirection);

    diffuse = max(0.0, dot(fs_in.normal, lightDirection));
    specular = max(0.0, dot(fs_in.normal, halfVector));

    attenuation = 1.0 /
        (light.constantAttenuation +
         light.linearAttenuation * lightDistance +
         light.quadraticAttenuation * lightDistance * lightDistance);
}

void spotLight(
    Light light,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 lightDirection = light.position - fs_in.position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;

    float spotCos = dot(lightDirection, -light.direction);
    // otherwise...
    attenuation = 1.0 /
        (light.constantAttenuation +
         light.linearAttenuation * lightDistance +
         light.quadraticAttenuation * lightDistance * lightDistance);

    vec3 halfVector = normalize(lightDirection + cameraDirection);

    diffuse = max(0.0, dot(fs_in.normal, lightDirection));
    specular = max(0.0, dot(fs_in.normal, halfVector));

    if (light.spotExponent < light.spotCutOff)
        specular = 0.0;
    else
        specular *= pow(spotCos, light.spotExponent);
}

void main()
{
    uint lightmask_ = lightmask << 1;
    uint index = 0;
    while ((lightmask_ = lightmask_ >> 1) != 0) {
        if ((lightmask_ & 1) == 1) {
            Light light = lights[index];
            float specular = 0.0;
            float diffuse = 0.0;
            float attenuation = 0.0;
            switch (light.type) {
                case Directional:
                    directionalLight(light, specular, diffuse, attenuation);
                    break;
                case Point:
                    pointLight(light, specular, diffuse, attenuation);
                    break;
                case Spot:
                    spotLight(light, specular, diffuse, attenuation);
                    break;
            }
            scatteredLight += light.ambient + light.color * diffuse * attenuation;
            reflectedLight += (((shininess + 8.0) / 8.0) * light.color) * pow(specular, shininess) * diffuse * attenuation;
        }
        index += 1;
    }

    vec4 color = diffuseFactor;
    if (diffuseIsTexture)
        color = texture(diffuseTexture, fs_in.texCoord);

    vec3 strength = specularFactor;
    if (specularIsTexture)
        strength = texture(specularTexture, fs_in.texCoord).xyz;

    vec3 rgb = color.rgb * scatteredLight + reflectedLight * strength;
    FragColor = vec4(rgb, color.a);
    BrightColor = vec4(rgb * 4.0 * smoothstep(bloomThreshold.x, bloomThreshold.y, dot(rgb, RGB_TO_LUM)), 1.0);
}
