#include "WasserXR/ecs/Scene.h"
#include "glad/gl.h"

#include "WasserXR/ecs/logging.h"
#include <GLFW/glfw3.h>
#include <stdio.h>
#include <stdlib.h>

WXR_System_Groups wxr_groups_wxr_window_pre_renderer = 1;

WXR_System_Groups wxr_select_wxr_window_pre_renderer(const WXR_Scene *scene,
                                                     const WXR_Entity entity) {
  if (wxr_entity_get_component(scene, entity, "WXR_Window")) {
    return 0;
  }
  return -1;
}

void wxr_system_wxr_window_pre_renderer(WXR_Scene *scene, WXR_Entity **entities,
                                        const size_t *groups) {
  for (size_t i = 0; i < *groups; i++) {
    void *window =
        wxr_entity_get_component(scene, entities[0][i], "WXR_Window");

    GLFWwindow *glfw_window = (GLFWwindow *)wxr_get(
        scene, window,
        "window"); // Discard const qualifier because direct access is needed
    if (!glfw_window) {
      wxr_warn("Window attribute is NULL");
      continue;
    }
    if (!glfwWindowShouldClose(glfw_window)) {
      glClearColor(0.1F, 0.1F, 0.1F, 1.0F);
      glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
    } else {
      wxr_remove_component(scene, entities[0][i], "WXR_Window");
    }
  }
}

WXR_System_Groups wxr_groups_wxr_window_post_renderer = 1;

WXR_System_Groups wxr_select_wxr_window_post_renderer(const WXR_Scene *scene,
                                                      const WXR_Entity entity) {
  if (wxr_entity_get_component(scene, entity, "WXR_Window")) {
    return 0;
  }
  return -1;
}

void wxr_system_wxr_window_post_renderer(WXR_Scene *scene,
                                         WXR_Entity **entities,
                                         const size_t *groups) {
  for (size_t i = 0; i < *groups; i++) {
    void *window =
        wxr_entity_get_component(scene, entities[0][i], "WXR_Window");

    GLFWwindow *glfw_window = (GLFWwindow *)wxr_get(
        scene, window,
        "window"); // Discard const qualifier because direct access is needed
    if (!glfw_window) {
      wxr_warn("Window attribute is NULL");
      continue;
    }
    if (!glfwWindowShouldClose(glfw_window)) {
      glfwSwapBuffers(glfw_window);
      glfwPollEvents();
    } else {
      wxr_remove_component(scene, entities[0][i], "WXR_Window");
    }
  }
}

WXR_System_Groups wxr_groups_wxr_window_quiter = 1;

WXR_System_Groups wxr_select_wxr_window_quiter(const WXR_Scene *scene,
                                               const WXR_Entity entity) {
  if (wxr_entity_get_component(scene, entity, "WXR_Window")) {
    return 0;
  }
  return -1;
}

void wxr_system_wxr_window_quiter(WXR_Scene *scene, WXR_Entity **entities,
                                  const size_t *groups) {
  if (*groups == 0) {
    wxr_set_scene_terminate(scene);
  }
}

WXR_System_Groups wxr_groups_wxr_window_reloader = 1;

WXR_System_Groups wxr_select_wxr_window_reloader(const WXR_Scene *scene,
                                                 const WXR_Entity entity) {
  if (wxr_entity_get_component(scene, entity, "WXR_Window")) {
    return 0;
  }
  return -1;
}

void wxr_system_wxr_window_reloader(WXR_Scene *scene, WXR_Entity **entities,
                                    const size_t *groups) {
  for (size_t i = 0; i < *groups; i++) {
    void *window =
        wxr_entity_get_component(scene, entities[0][i], "WXR_Window");

    GLFWwindow *glfw_window = (GLFWwindow *)wxr_get(
        scene, window,
        "window"); // Discard const qualifier because direct access is needed
    if (!glfw_window) {
      wxr_warn("Window attribute is NULL");
      continue;
    }
    if (glfwGetKey(glfw_window, GLFW_KEY_R) == GLFW_PRESS) {
      wxr_set_scene_reload(scene);
      return;
    }
  }
}
