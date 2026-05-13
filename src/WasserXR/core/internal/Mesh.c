#include <glad/gl.h>

#include "Mesh_internal.h"
#include "WasserXR/ecs/logging.h"
#include <assimp/cimport.h>
#include <assimp/postprocess.h>
#include <assimp/scene.h>
#include <glib.h>
#include <stdlib.h>

WXR_Mesh *wxr_create_mesh_from_data(WXR_Mesh_Data *mesh_data) {
  WXR_Mesh *mesh = (WXR_Mesh *)malloc(sizeof(WXR_Mesh));
  wxr_assert(mesh, "Malloc returned NULL during wxr_create_mesh_from_data");

  mesh->numIndices = (int)mesh_data->faces_size * 3;

  // Generate the buffers
  glGenVertexArrays(1, &mesh->vao);
  wxr_assert(mesh->vao, "Vertex Array couldn't be allocated");
  glGenBuffers(1, &mesh->vertexVbo);
  wxr_assert(mesh->vertexVbo, "Vertex Buffer couldn't be allocated");
  glGenBuffers(1, &mesh->normalVbo);
  wxr_assert(mesh->normalVbo, "Normal Buffer couldn't be allocated");
  glGenBuffers(1, &mesh->uvVbo);
  wxr_assert(mesh->uvVbo, "UV Buffer couldn't be allocated");
  glGenBuffers(1, &mesh->ebo);
  wxr_assert(mesh->ebo, "Element Buffer couldn't be allocated");

  // // Bind the buffers
  glBindVertexArray(mesh->vao);

  // Move vertices over
  glBindBuffer(GL_ARRAY_BUFFER, mesh->vertexVbo);
  glBufferData(GL_ARRAY_BUFFER,
               (long)sizeof(float) * 3 * (long)mesh_data->vertices_size,
               mesh_data->vertices, GL_STATIC_DRAW);
  glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 3 * sizeof(float), (void *)0);
  glEnableVertexAttribArray(0);

  // Upload normals
  glBindBuffer(GL_ARRAY_BUFFER, mesh->normalVbo);
  glBufferData(GL_ARRAY_BUFFER,
               (long)sizeof(float) * 3 * (long)mesh_data->vertices_size,
               mesh_data->normals, GL_STATIC_DRAW);
  glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, 3 * sizeof(float), (void *)0);
  glEnableVertexAttribArray(1);

  // Upload UVs
  glBindBuffer(GL_ARRAY_BUFFER, mesh->uvVbo);
  glBufferData(GL_ARRAY_BUFFER,
               (long)sizeof(float) * 2 * (long)mesh_data->vertices_size,
               mesh_data->uvs, GL_STATIC_DRAW);
  glVertexAttribPointer(2, 2, GL_FLOAT, GL_FALSE, 2 * sizeof(float), (void *)0);
  glEnableVertexAttribArray(2);

  glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, mesh->ebo);
  glBufferData(GL_ELEMENT_ARRAY_BUFFER,
               (long)sizeof(unsigned int) * 3 * mesh_data->faces_size,
               mesh_data->indices, GL_STATIC_DRAW);

  // Unbind
  glBindBuffer(GL_ARRAY_BUFFER, 0);
  // You are not allowed to unbind the ebo because it is stored in the vao
  // directly
  glBindVertexArray(0);
  return mesh;
}

void wxr_destroy_mesh(WXR_Mesh *mesh) {
  glDeleteVertexArrays(1, &mesh->vao);
  glDeleteBuffers(1, &mesh->vertexVbo);
  glDeleteBuffers(1, &mesh->normalVbo);
  glDeleteBuffers(1, &mesh->uvVbo);
  glDeleteBuffers(1, &mesh->ebo);
  free(mesh);
}
