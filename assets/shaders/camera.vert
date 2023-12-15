#version 430 core

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aTexCoord;
layout (location = 3) in vec4 aTangent;
layout (location = 4) in mat4 model_matrix;

uniform mat4 view_matrix;
uniform mat4 projection_matrix;

out VS_OUT {
    vec3 normal;
    vec2 texCoord;
    vec4 tangent;
} vs_out;

void main()
    {
        gl_Position = projection_matrix * view_matrix * model_matrix * vec4(aPos, 1.0);
        vs_out.texCoord = aTexCoord;
        vs_out.normal = aNormal;
        vs_out.tangent = aTangent;
    }
