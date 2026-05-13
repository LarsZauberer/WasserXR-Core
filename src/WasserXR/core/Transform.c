#include "WasserXR/ecs/Macros.h"
#include "WasserXR/ecs/Scene.h"
#include "WasserXR/ecs/logging.h"
#include "cglm/vec3.h"
#include <stdlib.h>

typedef struct WXR_Transform {
  vec3 position;
  vec3 rotation;
  vec3 scale;
} WXR_Transform;

void *wxr_create_WXR_Transform() {
  WXR_Transform *ptr = (WXR_Transform *)malloc(sizeof(WXR_Transform));
  wxr_assert_abort_value(ptr, NULL,
                         "Malloc failed during wxr_create_WXR_Transform");

  glm_vec3_zero(ptr->position);
  glm_vec3_zero(ptr->rotation);
  glm_vec3_one(ptr->scale);

  return ptr;
}

void wxr_destroy_WXR_Transform(void *ptr) { free(ptr); }

WXR_BASIC_SERIALIZERS(WXR_Transform, x, &component->position[0], sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, y, &component->position[1], sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, z, &component->position[2], sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, rx, &component->rotation[0],
                      sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, ry, &component->rotation[1],
                      sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, rz, &component->rotation[2],
                      sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, sx, &component->scale[0], sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, sy, &component->scale[1], sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Transform, sz, &component->scale[2], sizeof(float));

WXR_BASIC_ACCESS(WXR_Transform, x, &component->position[0], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, y, &component->position[1], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, z, &component->position[2], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, rx, &component->rotation[0], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, ry, &component->rotation[1], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, rz, &component->rotation[2], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, sx, &component->scale[0], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, sy, &component->scale[1], sizeof(float));
WXR_BASIC_ACCESS(WXR_Transform, sz, &component->scale[2], sizeof(float));

void wxr_schema_WXR_Transform(WXR_Component_Schema *schema) {
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, x);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, y);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, z);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, rx);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, ry);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, rz);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, sx);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, sy);
  WXR_SCHEMA_FIELD_FULL(WXR_Transform, WXR_F, sz);
}
