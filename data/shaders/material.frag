#version 430 core

out vec4 FragColor;

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

void ambientLight(
    vec3 ambient
) {
    scatteredLight += ambient;
}

void directionalLight(
    vec3 ambient,
    vec3 direction,
    vec3 color
) {
    vec3 halfVector = normalize(-direction + cameraDirection);
    float diffuse = max(0.0, dot(fs_in.normal, direction));
    float specular = max(0.0, dot(fs_in.normal, halfVector));

    if (diffuse == 0.0)
        specular = 0.0;
    else
        specular = pow(specular, shininess);

    scatteredLight += ambient + color * diffuse; // We'll multiply in the material's diffuse color at the last step thanks to the distributive property
    reflectedLight += color * specular; // We'll multiply in the material's specular strength in last too
}

void pointLight(
    vec3 ambient,
    vec3 position,
    vec3 color,
    float constantAttenuation,
    float linearAttenuation,
    float quadraticAttenuation
) {
    vec3 lightDirection = position - fs_in.position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;
    vec3 halfVector = normalize(lightDirection + cameraDirection);

    float diffuse = max(0.0, dot(fs_in.normal, lightDirection));
    float specular = max(0.0, dot(fs_in.normal, halfVector));

    float attenuation = 1.0 /
        (constantAttenuation +
         linearAttenuation * lightDistance +
         quadraticAttenuation * lightDistance * lightDistance);

    if (diffuse == 0.0)
        specular = 0.0;
    else
        specular = pow(specular, shininess);

    scatteredLight += ambient + color * diffuse * attenuation;
    reflectedLight += color * specular * attenuation;
}

void spotLight(
    vec3 ambient,
    vec3 position,
    vec3 direction,
    vec3 color,
    float cutoff,
    float spotExp,
    float constantAttenuation,
    float linearAttenuation,
    float quadraticAttenuation
) {
    vec3 lightDirection = position - fs_in.position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;

    float spotCos = dot(lightDirection, -direction);
    // otherwise...
    float attenuation = 1.0 /
        (constantAttenuation +
         linearAttenuation * lightDistance +
         quadraticAttenuation * lightDistance * lightDistance);

    if (spotCos < cutoff)
        attenuation = 0.0;
    //else
        //attenuation *= pow(spotCos, spotExp);

    float intensity = 1.0 - (1.0 - spotCos)/(1.0 - cutoff);

    vec3 halfVector = normalize(lightDirection + cameraDirection);

    float diffuse = max(0.0, dot(fs_in.normal, lightDirection));
    float specular = max(0.0, dot(fs_in.normal, halfVector));

    if (diffuse == 0.0)
        specular = 0.0;
    else
        specular = pow(specular, shininess);

    scatteredLight += ambient + intensity * color * diffuse * attenuation;
    reflectedLight += intensity * color * specular * attenuation;
}

void main()
{
    uint lightmask_ = lightmask << 1;
    uint index = 0;
    while ((lightmask_ = lightmask_ >> 1) != 0) {
        if ((lightmask_ & 1) == 1) {
            Light light = lights[index];
            switch (light.type) {
                case Directional:
                    directionalLight(light.ambient, light.direction, light.color);
                    break;
                case Point:
                    pointLight(light.ambient, light.position, light.color,
                               light.constantAttenuation,
                               light.linearAttenuation,
                               light.quadraticAttenuation);
                    break;
                case Spot:
                    spotLight(light.ambient, light.position, light.direction,
                              light.color,
                              light.spotCutOff,
                              light.spotExponent,
                              light.constantAttenuation,
                              light.linearAttenuation,
                              light.quadraticAttenuation);
                    break;
                default:
                    ambientLight(light.ambient);
                    break;
            }
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
}
