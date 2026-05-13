#include <WasserXR/ecs/Macros.h>
#include <WasserXR/ecs/Scene.h>
#include <WasserXR/ecs/logging.h>
#include <stdlib.h>

typedef struct WXR_Camera {
  float fov;
  float near;
  float far;
} WXR_Camera;

void *wxr_create_WXR_Camera() {
  WXR_Camera *cam = (WXR_Camera *)malloc(sizeof(WXR_Camera));
  wxr_assert(cam, "Malloc failed during wxr_create_WXR_Camera");

  cam->fov = 90.0F;
  cam->near = 0.1F;
  cam->far = 100.0F;

  return cam;
}

WXR_BASIC_SERIALIZERS(WXR_Camera, fov, &component->fov, sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Camera, near, &component->near, sizeof(float));
WXR_BASIC_SERIALIZERS(WXR_Camera, far, &component->far, sizeof(float));

WXR_BASIC_ACCESS(WXR_Camera, fov, &component->fov, sizeof(float));
WXR_BASIC_ACCESS(WXR_Camera, near, &component->near, sizeof(float));
WXR_BASIC_ACCESS(WXR_Camera, far, &component->far, sizeof(float));

void wxr_destroy_WXR_Camera(void *cam) { free(cam); }

void wxr_schema_WXR_Camera(WXR_Component_Schema *schema) {
  WXR_SCHEMA_FIELD_FULL(WXR_Camera, WXR_F, fov);
  WXR_SCHEMA_FIELD_FULL(WXR_Camera, WXR_F, near);
  WXR_SCHEMA_FIELD_FULL(WXR_Camera, WXR_F, far);
}
