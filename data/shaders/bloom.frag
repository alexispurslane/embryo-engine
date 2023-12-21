#version 430 core

layout (binding = 0, rgba16f) uniform image2D hdrImage;
layout (binding = 0, rgba16f) uniform image2D blurImage;

uniform float sceneFactor = 1.0;
uniform float bloomFactor = 1.0;

out vec4 FragColor;

void main() {
    vec4 color = vec4(0.0);

    FragColor = imageLoad(hdrImage, ivec2(gl_FragCoord.xy)) * sceneFactor
        + imageLoad(blurImage, ivec2(gl_FragCoord.xy)) * bloomFactor;
}
