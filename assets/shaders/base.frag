#version 330 core
out vec4 FragColor;

in vec3 Normal;

void main()
{
    FragColor = vec4(Normal, 1.0) * vec4(1.0f, 1.0f, 1.0f, 1.0f);
} 
