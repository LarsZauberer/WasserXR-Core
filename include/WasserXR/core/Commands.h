// This is not a system
#include "WasserXR/ecs/Scene.h"

typedef void (*WXR_Command_Function)(char **args, WXR_Scene *scene);

typedef struct {
  const char *command;
  WXR_Command_Function func;
} WXR_Command;

void wxr_command_reload(char **args, WXR_Scene *scene);
void wxr_command_exit(char **args, WXR_Scene *scene);
void wxr_command_addEntity(char **args, WXR_Scene *scene);
void wxr_command_removeEntity(char **args, WXR_Scene *scene);
void wxr_command_addComponent(char **args, WXR_Scene *scene);
void wxr_command_get(char **args, WXR_Scene *scene);
void wxr_command_set(char **args, WXR_Scene *scene);
void wxr_command_addSystem(char **args, WXR_Scene *scene);
void wxr_command_removeSystem(char **args, WXR_Scene *scene);
void wxr_command_loadPlugin(char **args, WXR_Scene *scene);
void wxr_command_unloadPlugin(char **args, WXR_Scene *scene);
void wxr_command_showEntities(char **args, WXR_Scene *scene);
void wxr_command_showPlugins(char **args, WXR_Scene *scene);
void wxr_command_showComponents(char **args, WXR_Scene *scene);
void wxr_command_showSystems(char **args, WXR_Scene *scene);
void wxr_command_save(char **args, WXR_Scene *scene);
void wxr_command_load(char **args, WXR_Scene *scene);

WXR_Command *wxr_create_command_list(size_t *size);
void wxr_destroy_command_list(WXR_Command *ptr);
