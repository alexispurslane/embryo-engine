#version 430 core

#define RGB_TO_LUM vec3(0.2125, 0.7154, 0.0721)

layout (binding = 0, r16f) uniform readonly image1D lumAvg;
layout (binding = 1, rgba16f) uniform readonly image2D hdrImage;

uniform vec4 params;
// params.x = L_white

out vec4 FragColor;

float curve(float L_in) {
    float simple = L_in / (1.0 + L_in);
    float extended = simple * (1.0 + L_in / (params.x * params.x));
    return extended;
}

float tone_adjust_single(float L_in, float L_avg) {
    // Adjust the exposure of that luminance based on current scene average
    // luminance
    float L_prime = L_in / (9.6 * L_avg);
    // adjusted display luminance
    float L_d = curve(L_prime);
    // Adjust the input pixel to the display luminance based on its original
    // (exposure adjusted) luminance
    return L_d / L_prime;
}

vec3 tone_mapping_y(vec3 C_in, float L_avg) {
    // Luminance of this pixel
    vec3 L_in = C_in * RGB_TO_LUM;

    return vec3(
        C_in.x * tone_adjust_single(L_in.x, L_avg),
        C_in.y * tone_adjust_single(L_in.y, L_avg),
        C_in.z * tone_adjust_single(L_in.z, L_avg)
    );
}

void main() {
    vec4 C_in = imageLoad(hdrImage, ivec2(gl_FragCoord.xy));
    float L_avg = imageLoad(lumAvg, 0).r * 100.0;

    C_in.rgb = tone_mapping_y(C_in.rgb, L_avg);

    FragColor = C_in;
}
