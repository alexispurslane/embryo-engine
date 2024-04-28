/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

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

layout (binding = 0, rgba16f) uniform readonly image2D gPosition;
layout (binding = 1, rgba16f) uniform readonly image2D gNormal;
layout (binding = 2, rgba16f) uniform readonly image2D gDiffuseColor;
layout (binding = 3, rgba16f) uniform readonly image2D gSpecShininess;
uniform vec2 bloomThreshold = vec2(0.0, 1.2);

layout (binding = 4, std140) uniform Light {
    vec3 position; // used in point and spot lights
    vec3 direction; // used in directional and spot lights
    float constantAttenuation; // used in point and spot lights
    vec3 ambient;
    float linearAttenuation; // used in point and spot lights
    vec3 color;
    float quadraticAttenuation; // used in point and spot lights
    float spotCutOff; // used in spot lights
    float spotExponent; // used in spot lights
} light;

uniform vec3 cameraDirection;

subroutine void RenderLight(
    vec3 position,
    vec3 normal,
    out float specular,
    out float diffuse,
    out float attenuation
);

layout(location = 0) subroutine uniform RenderLight renderLight;

layout(index = 0) subroutine(RenderLight) void ambientLight(
    vec3 position,
    vec3 normal,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    diffuse = 1.0;
    specular = 1.0;
    attenuation = 1.0;
}

layout(index = 1) subroutine(RenderLight) void directionalLight(
    vec3 position,
    vec3 normal,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 halfVector = normalize(-light.direction + cameraDirection);
    diffuse = max(0.0, dot(normal, light.direction));
    specular = max(0.0, dot(normal, halfVector));
    attenuation = 1.0;
}

layout(index = 2) subroutine(RenderLight) void pointLight(
    vec3 position,
    vec3 normal,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 lightDirection = light.position - position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;
    vec3 halfVector = normalize(lightDirection + cameraDirection);

    diffuse = max(0.0, dot(normal, lightDirection));
    specular = max(0.0, dot(normal, halfVector));

    attenuation = 1.0 /
        (light.constantAttenuation +
         light.linearAttenuation * lightDistance +
         light.quadraticAttenuation * lightDistance * lightDistance);
}

layout(index = 3) subroutine(RenderLight) void spotLight(
    vec3 position,
    vec3 normal,
    out float specular,
    out float diffuse,
    out float attenuation
) {
    vec3 lightDirection = light.position - position.xyz;
    float lightDistance = length(lightDirection);
    lightDirection = lightDirection / lightDistance;

    float spotCos = dot(lightDirection, -light.direction);
    // otherwise...
    attenuation = 1.0 /
        (light.constantAttenuation +
         light.linearAttenuation * lightDistance +
         light.quadraticAttenuation * lightDistance * lightDistance);

    vec3 halfVector = normalize(lightDirection + cameraDirection);

    diffuse = max(0.0, dot(normal, lightDirection));
    specular = max(0.0, dot(normal, halfVector));

    if (light.spotExponent < light.spotCutOff)
        specular = 0.0;
    else
        specular *= pow(spotCos, light.spotExponent);
}

void main()
{
    vec3 scatteredLight = vec3(0.0); // ambient and diffuse, color is a mix of object and light
    vec3 reflectedLight = vec3(0.0); // specular, color is based on light alone

    float specular = 0.0;
    float diffuse = 0.0;
    float attenuation = 0.0;

    renderLight(
        imageLoad(gPosition, ivec2(gl_FragCoord.xy)).rgb,
        imageLoad(gNormal, ivec2(gl_FragCoord.xy)).rgb,
        specular,
        diffuse,
        attenuation
    );

    vec4 specShininess = imageLoad(gSpecShininess, ivec2(gl_FragCoord.xy));
    vec3 diffuseColor = imageLoad(gDiffuseColor, ivec2(gl_FragCoord.xy)).rgb;

    scatteredLight += light.ambient + light.color * diffuse * attenuation;
    reflectedLight += (((specShininess.a + 8.0) / 8.0) * light.color) * pow(specular, specShininess.a) * diffuse * attenuation;

    vec3 rgb = diffuseColor * scatteredLight + reflectedLight * specShininess.rgb;
    FragColor = vec4(rgb, 1.0);
    BrightColor = vec4(rgb * 4.0 * smoothstep(bloomThreshold.x, bloomThreshold.y, dot(rgb, RGB_TO_LUM)), 1.0);
}
