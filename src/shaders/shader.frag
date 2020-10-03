#version 100
varying highp vec2 v_uv;
varying lowp vec4 v_color;

uniform sampler2D u_texture;

void main()
{
    gl_FragColor = texture2D(u_texture, v_uv) * v_color;
}