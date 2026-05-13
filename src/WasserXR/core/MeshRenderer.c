#include "Mesh_internal.h"
#include "glad/gl.h"

#include "GLFW/glfw3.h"
#include "Shader_internal.h"
#include "WasserXR/core/Model.h"
#include "WasserXR/ecs/Scene.h"
#include "WasserXR/ecs/logging.h"
#include "cglm/affine-pre.h"
#include "cglm/affine.h"
#include "cglm/cam.h"
#include "cglm/mat4.h"
#include "cglm/types.h"
#include "cglm/util.h"
#include "cglm/vec4.h"
#include <stdio.h>

WXR_System_Groups wxr_groups_wxr_mesh_renderer = 3;

WXR_System_Groups wxr_select_wxr_mesh_renderer(const WXR_Scene *scene,
                                               const WXR_Entity entity) {
  size_t normal_object =
      wxr_entity_get_component(scene, entity, "WXR_Transform") &&
      wxr_entity_get_component(scene, entity, "WXR_Model");
  size_t camera_object =
      wxr_entity_get_component(scene, entity, "WXR_Transform") &&
      wxr_entity_get_component(scene, entity, "WXR_Camera");
  size_t window = (size_t)wxr_entity_get_component(scene, entity, "WXR_Window");
  if (window) {
    return 0;
  }
  if (camera_object) {
    return 1;
  }
  if (normal_object) {
    return 2;
  }
  return -1;
}

void wxr_system_wxr_mesh_renderer(WXR_Scene *scene, WXR_Entity **entities,
                                  const size_t *sizes) {
  WXR_Entity camera_entity;
  void *camera;
  void *cam_transform;

  WXR_Entity window_entity;
  void *window;

  if (!sizes[0]) {
    wxr_warn("No window!\n");
    return;
  }

  if (!sizes[1]) {
    wxr_warn("No camera!\n");
    return;
  }

  window_entity = entities[0][0];
  window = wxr_entity_get_component(scene, window_entity, "WXR_Window");
  GLFWwindow *glfw_window = (GLFWwindow *)wxr_get(
      scene, window,
      "window"); // Const qualifier discard because we need direct access
  if (!glfw_window) {
    wxr_warn("Window field is NULL");
    return;
  }

  camera_entity = entities[1][0];
  cam_transform =
      wxr_entity_get_component(scene, camera_entity, "WXR_Transform");
  camera = wxr_entity_get_component(scene, camera_entity, "WXR_Camera");

  for (size_t i = 0; i < sizes[2]; i++) {
    // Normal mesh entity
    WXR_Entity entity = entities[2][i];

    WXR_Model *model = wxr_entity_get_component(scene, entity, "WXR_Model");
    void *transform = wxr_entity_get_component(scene, entity, "WXR_Transform");

    const WXR_Mesh **meshes =
        (const WXR_Mesh **)wxr_get(scene, model, "meshes");
    const unsigned int num_meshes =
        *(const unsigned int *)wxr_get(scene, model, "num_meshes");
    const WXR_Shader *shader = wxr_get(scene, model, "shader");

    // Check if the model is loaded yet
    if (meshes == NULL || shader == NULL) {
      wxr_warn("Model is not properly loaded.");
      continue;
    }

    mat4 model_transform;
    mat4 view_transform;
    mat4 projection_transform;
    glm_mat4_identity(model_transform);
    glm_mat4_identity(view_transform);
    glm_mat4_identity(projection_transform);

    // Create the transformation matrix

    // World Space placement
    vec3 position = {*(float *)wxr_get(scene, transform, "x"),
                     *(float *)wxr_get(scene, transform, "y"),
                     *(float *)wxr_get(scene, transform, "z")};
    vec3 scale = {*(float *)wxr_get(scene, transform, "sx"),
                  *(float *)wxr_get(scene, transform, "sy"),
                  *(float *)wxr_get(scene, transform, "sz")};
    glm_translate(model_transform, position);
    glm_rotate_x(model_transform,
                 glm_rad(*(float *)wxr_get(scene, transform, "rx")),
                 model_transform);
    glm_rotate_y(model_transform,
                 glm_rad(*(float *)wxr_get(scene, transform, "ry")),
                 model_transform);
    glm_rotate_z(model_transform,
                 glm_rad(*(float *)wxr_get(scene, transform, "rz")),
                 model_transform);
    glm_scale(model_transform, scale);

    // Camera placement
    vec3 camera_position = {*(float *)wxr_get(scene, cam_transform, "x"),
                            *(float *)wxr_get(scene, cam_transform, "y"),
                            *(float *)wxr_get(scene, cam_transform, "z")};
    vec4 camera_pos_4;
    glm_vec4(camera_position, 1.0F, camera_pos_4);
    glm_vec4_negate(camera_pos_4);
    glm_translate(view_transform, camera_pos_4);
    glm_rotate_x(view_transform,
                 glm_rad(*(float *)wxr_get(scene, cam_transform, "rx")),
                 view_transform);
    glm_rotate_y(view_transform,
                 glm_rad(*(float *)wxr_get(scene, cam_transform, "ry")),
                 view_transform);
    glm_rotate_z(view_transform,
                 glm_rad(*(float *)wxr_get(scene, cam_transform, "rz")),
                 view_transform);

    // Perspective
    int width;
    int height;
    float fov = *(float *)wxr_get(scene, camera, "fov");
    float near = *(float *)wxr_get(scene, camera, "near");
    float far = *(float *)wxr_get(scene, camera, "far");
    glfwGetWindowSize(glfw_window, &width, &height);
    glm_perspective(glm_rad(fov), (float)width / (float)height, near, far,
                    projection_transform);

    int status = wxr_use_shader(shader);
    if (status) {
      wxr_warn("Shader couldn't be applied to the mesh of entity %ld. Skipping "
               "rendering of entity",
               entity);
      continue;
    }

    // Put everything to the respective uniforms in the shader
    wxr_set_shader_uniform_mat4(shader, "model", model_transform);
    wxr_set_shader_uniform_mat4(shader, "view", view_transform);
    wxr_set_shader_uniform_mat4(shader, "projection", projection_transform);

    // Draw the meshes
    for (unsigned int i = 0; i < num_meshes; i++) {
      glBindVertexArray(meshes[i]->vao);

      glDrawElements(GL_TRIANGLES, meshes[i]->numIndices, GL_UNSIGNED_INT, 0);
    }
  }
}
