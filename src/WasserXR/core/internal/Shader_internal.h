#ifndef WXR_SHADER_INTERNAL_H
#define WXR_SHADER_INTERNAL_H

#include <cglm/cglm.h>

/**
 * @brief Opaque shader structure
 */
typedef struct WXR_Shader WXR_Shader;

/**
 * @brief Internal shader structure definition
 */
struct WXR_Shader {
  char *path;
  unsigned int vertex_shader;
  unsigned int fragment_shader;
  unsigned int program;
  int is_loaded;
  int is_compiled;
  char *vertex_source;
  char *fragment_source;
};

/**
 * @brief Creates a shader object on the heap
 * @param path Base path to the shader files (without .vert/.frag extension)
 * @return Pointer to the newly created shader object
 */
WXR_Shader *wxr_create_shader(const char *path);

/**
 * @brief Loads shader source code from filesystem
 * @param shader The shader object
 * @return 0 on success, 1 on failure
 */
int wxr_load_shader(WXR_Shader *shader);

/**
 * @brief Compiles the shader program
 * @param shader The shader object
 * @return 0 on success, 1 on failure
 */
int wxr_compile_shader(WXR_Shader *shader);

/**
 * @brief Activates the shader for use with OpenGL
 * @param shader The shader object
 * @return 0 on success, 1 on failure
 */
int wxr_use_shader(const WXR_Shader *shader);

/**
 * @brief Destroys the shader and frees all resources
 * @param shader The shader object
 */
void wxr_destroy_shader(WXR_Shader *shader);

/**
 * @brief Set a float uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The float value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_1f(const WXR_Shader *shader, const char *name,
                              float value);

/**
 * @brief Set an integer uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The integer value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_1i(const WXR_Shader *shader, const char *name,
                              int value);

/**
 * @brief Set a vec2 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The vec2 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_2f(const WXR_Shader *shader, const char *name,
                              const vec2 value);

/**
 * @brief Set a vec3 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The vec3 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_3f(const WXR_Shader *shader, const char *name,
                              const vec3 value);

/**
 * @brief Set a vec4 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The vec4 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_4f(const WXR_Shader *shader, const char *name,
                              const vec4 value);

/**
 * @brief Set a mat2 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The mat2 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_mat2(const WXR_Shader *shader, const char *name,
                                const mat2 value);

/**
 * @brief Set a mat3 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The mat3 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_mat3(const WXR_Shader *shader, const char *name,
                                const mat3 value);

/**
 * @brief Set a mat4 uniform in the shader
 * @param shader The shader object
 * @param name The uniform name
 * @param value The mat4 value
 * @return 0 on success, 1 on failure
 */
int wxr_set_shader_uniform_mat4(const WXR_Shader *shader, const char *name,
                                const mat4 value);

#endif
