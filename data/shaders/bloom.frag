/*
 * Copyright (C) 2023 Alexis Purslane <alexispurslane@pm.me>
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#version 430 core

layout (binding = 0, rgba16f) uniform image2D hdrImage;
uniform sampler2D blurImage;

uniform float sceneFactor = 1.0;
uniform float bloomFactor = 1.0;

out vec4 FragColor;

void main() {
    FragColor = imageLoad(hdrImage, ivec2(gl_FragCoord.xy)) * sceneFactor
        + texture(blurImage, ivec2(gl_FragCoord.xy)) * bloomFactor;
}
