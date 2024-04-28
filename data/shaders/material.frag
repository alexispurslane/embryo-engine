/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#version 430 core

layout (location = 0) out vec4 Position;
layout (location = 1) out vec4 Normal;
layout (location = 2) out vec4 DiffuseColor;
layout (location = 3) out vec4 SpecShininess;

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

void main()
{
    vec4 color = diffuseFactor;
    if (diffuseIsTexture)
        color = texture(diffuseTexture, fs_in.texCoord);

    vec3 strength = specularFactor;
    if (specularIsTexture)
        strength = texture(specularTexture, fs_in.texCoord).xyz;

    Position = fs_in.position;
    Normal = vec4(fs_in.normal, 0.0);
    DiffuseColor = color;
    SpecShininess = vec4(strength, shininess);
}
