#include "WasserXR/core/Model.h"
#include "Mesh_internal.h"
#include "Shader_internal.h"
#include "WasserXR/ecs/Macros.h"
#include "WasserXR/ecs/Scene.h"
#include "WasserXR/ecs/logging.h"
#include "WasserXR/ecs/utils.h"
#include <glad/gl.h>
#include <stdlib.h>
#include <string.h>

struct WXR_Model {
  char *model_name;
  char *shader_name;
  WXR_Shader *shader;
  unsigned int numMeshes;
  WXR_Mesh **meshes;
};

void *wxr_create_WXR_Model() {
  WXR_Model *model = (WXR_Model *)malloc(sizeof(WXR_Model));
  wxr_assert(model, "Malloc failed during wxr_create_WXR_Model");

  model->model_name = NULL;
  model->shader_name = NULL;
  model->meshes = NULL;
  model->shader = NULL;
  model->numMeshes = 0;

  return model;
}

void wxr_destroy_WXR_Model(void *ptr) {
  WXR_Model *model = (WXR_Model *)ptr;
  free(model->model_name);
  free(model->shader_name);

  // Free all the meshes
  if (model->meshes) {
    for (unsigned int i = 0; i < model->numMeshes; i++) {
      wxr_destroy_mesh(model->meshes[i]);
    }
    free(model->meshes);
  }

  // Free the shader
  wxr_destroy_shader(model->shader);

  // Free the model
  free(model);
}

void wxr_set_WXR_Model_model_name(void *component, const void *data) {
  WXR_Model *model = (WXR_Model *)component;
  const char *path = (const char *)data;
  if (path) {
    // Replace the field
    free(model->model_name);
    model->model_name = wxr_copy_char_ptr(path);

    // Save the old numMeshes for later to destroy the old meshes
    unsigned int old_numMeshes = model->numMeshes;

    // Read all the mesh data (array)
    WXR_Mesh_Data *mesh_data =
        wxr_read_mesh_data(&model->numMeshes, model->model_name);
    if (!mesh_data) {
      wxr_warn("Failed to read the mesh data of `%s`", model->model_name);
      model->numMeshes = old_numMeshes;
      return;
    }

    // Array of pointers
    WXR_Mesh **meshes =
        (WXR_Mesh **)malloc(sizeof(WXR_Mesh *) * model->numMeshes);
    wxr_assert(meshes, "Malloc failed during creation of the meshes array in "
                       "wxr_activate_WXR_Model");

    // Load the mesh with opengl
    for (unsigned int i = 0; i < model->numMeshes; i++) {
      meshes[i] = wxr_create_mesh_from_data(&mesh_data[i]);
      // Free up the mesh data
      wxr_destroy_mesh_data(&mesh_data[i]);
    }
    free(mesh_data);

    // Clear old meshes
    for (unsigned int i = 0; i < old_numMeshes; i++) {
      wxr_destroy_mesh(model->meshes[i]);
    }
    free(model->meshes);

    model->meshes = meshes;
  }
}

void wxr_set_WXR_Model_shader_name(void *component, const void *data) {
  WXR_Model *model = (WXR_Model *)component;
  const char *path = (const char *)data;
  if (path) {
    // Replace the field
    free(model->shader_name);
    model->shader_name = wxr_copy_char_ptr(path);

    // Unload old shader
    wxr_destroy_shader(model->shader);

    model->shader = wxr_create_shader(model->shader_name);
    int status = wxr_load_shader(model->shader);
    if (status) {
      wxr_warn("Failed to load the shader: %s", model->shader_name);
    } else {
      status = wxr_compile_shader(model->shader);
      if (status) {
        wxr_warn("Failed to compile the shader: %s", model->shader_name);
      }
    }
  }
}

WXR_STRING_SERIALIZE(WXR_Model, model_name, component->model_name);
WXR_SET_DESERIALIZE(WXR_Model, model_name, component->model_name,
                    wxr_set_WXR_Model_model_name);
WXR_STRING_SERIALIZE(WXR_Model, shader_name, component->shader_name);
WXR_SET_DESERIALIZE(WXR_Model, shader_name, component->shader_name,
                    wxr_set_WXR_Model_shader_name);

WXR_STRING_GETTER(WXR_Model, shader_name, component->shader_name);
WXR_STRING_GETTER(WXR_Model, model_name, component->model_name);

WXR_BASIC_GETTER(WXR_Model, shader, component->shader, sizeof(WXR_Shader *));
WXR_BASIC_GETTER(WXR_Model, meshes, component->meshes, sizeof(WXR_Mesh **));

WXR_BASIC_GETTER(WXR_Model, num_meshes, &component->numMeshes,
                 sizeof(unsigned int));

void wxr_schema_WXR_Model(WXR_Component_Schema *schema) {
  WXR_SCHEMA_FIELD_FULL(WXR_Model, WXR_S, model_name);
  WXR_SCHEMA_FIELD_FULL(WXR_Model, WXR_S, shader_name);

  WXR_SCHEMA_FIELD_GET(WXR_Model, WXR_BLOB_ARRAY, meshes);
  WXR_SCHEMA_FIELD_GET(WXR_Model, WXR_BLOB, shader);
  WXR_SCHEMA_FIELD_GET(WXR_Model, WXR_L, num_meshes);
}
