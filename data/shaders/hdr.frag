#version 430 core

#define RGB_TO_LUM vec3(0.2125, 0.7154, 0.0721)

layout (binding = 0, r16f) uniform readonly image1D lumAvg;
layout (binding = 1, rgba16f) uniform readonly image2D hdrImage;

uniform vec4 params;
// params.x = L_white

out vec4 FragColor;

float f1(float hdrMax, float contrast, float shoulder, float midIn, float midOut) {
    return
        -((-pow(midIn, contrast) + (midOut * (pow(hdrMax, contrast * shoulder) * pow(midIn, contrast) -
            pow(hdrMax, contrast) * pow(midIn, contrast * shoulder) * midOut)) /
            (pow(hdrMax, contrast * shoulder) * midOut - pow(midIn, contrast * shoulder) * midOut)) /
            (pow(midIn, contrast * shoulder) * midOut));
}

// General tonemapping operator, build 'c' term.
float f2(float hdrMax, float contrast, float shoulder, float midIn, float midOut) {
    return (pow(hdrMax, contrast * shoulder) * pow(midIn, contrast) - pow(hdrMax, contrast) * pow(midIn, contrast * shoulder) * midOut) /
           (pow(hdrMax, contrast * shoulder) * midOut - pow(midIn, contrast * shoulder) * midOut);
}

// General tonemapping operator, p := {contrast,shoulder,b,c}.
float f3(float x, vec4 p) {
    float z = pow(x, p.r);
    return z / (pow(z, p.g) * p.b + p.a);
}

vec3 lottes(vec3 color) {
    const float hdrMax = params.x; // How much HDR range before clipping. HDR modes likely need this pushed up to say 25.0.
    const float contrast = 1.2; // Use as a baseline to tune the amount of contrast the tonemapper has.
    const float shoulder = 1.0; // Likely don't need to mess with this factor, unless matching existing tonemapper is not working well..
    const float midIn = 0.18; // most games will have a {0.0 to 1.0} range for LDR so midIn should be 0.18.
    const float midOut = 0.18; // Use for LDR. For HDR10 10:10:10:2 use maybe 0.18/25.0 to start. For scRGB, I forget what a good starting point is, need to re-calculate.

    float b = f1(hdrMax, contrast, shoulder, midIn, midOut);
    float c = f2(hdrMax, contrast, shoulder, midIn, midOut);

    #define EPS 1e-6f
    float peak = max(color.r, max(color.g, color.b));
    peak = max(EPS, peak);

    vec3 ratio = color / peak;
    peak = f3(peak, vec4(contrast, shoulder, b, c) );
    // then process ratio

    // probably want send these pre-computed (so send over saturation/crossSaturation as a constant)
    float crosstalk = 4.0; // controls amount of channel crosstalk
    float saturation = contrast; // full tonal range saturation control
    float crossSaturation = contrast * 16.0; // crosstalk saturation

    float white = 1.0;

    // wrap crosstalk in transform
    ratio = pow(abs(ratio), vec3(saturation / crossSaturation));
    ratio = mix(ratio, vec3(white), vec3(pow(peak, crosstalk)));
    ratio = pow(abs(ratio), vec3(crossSaturation));

    // then apply ratio to peak
    color = peak * ratio;
    return color;
}

float reinhard(float L_in) {
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

    C_in.rgb = lottes(C_in.rgb);

    FragColor = C_in;
}
