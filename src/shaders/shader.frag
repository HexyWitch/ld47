#version 100
varying highp vec2 v_uv;
varying lowp vec4 v_color;

uniform sampler2D u_texture;

void main()
{
    // multiply color by alpha for correct blending
    highp vec4 color = texture2D(u_texture, v_uv) * v_color;
    color.rgb *= color.a;
    gl_FragColor = color;
}