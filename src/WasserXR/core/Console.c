#include "WasserXR/core/Commands.h"
#include "WasserXR/ecs/Macros.h"
#include "WasserXR/ecs/Scene.h"

#include <stdlib.h>
typedef struct WXR_Console {
  size_t command_list_size;
  WXR_Command *command_list;
} WXR_Console;

void *wxr_create_WXR_Console() {
  WXR_Console *console = (WXR_Console *)malloc(sizeof(WXR_Console));
  console->command_list = wxr_create_command_list(&console->command_list_size);
  return console;
}

void wxr_destroy_WXR_Console(void *ptr) {
  WXR_Console *console = (WXR_Console *)ptr;
  wxr_destroy_command_list(console->command_list);
  free(console);
}

WXR_BASIC_GETTER(WXR_Console, command_list_size, &component->command_list_size,
                 sizeof(size_t));
WXR_BASIC_GETTER(WXR_Console, command_list, component->command_list,
                 sizeof(WXR_Command *));

void wxr_schema_WXR_Console(WXR_Component_Schema *schema) {
  WXR_SCHEMA_FIELD_GET(WXR_Console, WXR_L, command_list_size);
  WXR_SCHEMA_FIELD_GET(WXR_Console, WXR_BLOB, command_list);
}
