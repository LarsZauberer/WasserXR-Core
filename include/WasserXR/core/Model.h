#ifndef WXR_MODEL_H
#define WXR_MODEL_H

#include "WasserXR/ecs/Scene.h"

typedef struct WXR_Model WXR_Model;

void *wxr_create_WXR_Model();
void wxr_destroy_WXR_Model(void *ptr);
void wxr_schema_WXR_Model(WXR_Component_Schema *schema);

#endif
