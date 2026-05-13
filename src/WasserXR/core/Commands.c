// This is not a component
#include "WasserXR/core/Commands.h"
#include "WasserXR/ecs/Scene.h"
#include "WasserXR/ecs/logging.h"
#include <stdlib.h>
#include <string.h>

WXR_Command *wxr_create_command_list(size_t *size) {
  *size = 17;
  WXR_Command *command_list =
      (WXR_Command *)malloc(sizeof(WXR_Command) * *size);

  command_list[0] = (WXR_Command){"reload", wxr_command_reload};
  command_list[1] = (WXR_Command){"exit", wxr_command_exit};
  command_list[2] = (WXR_Command){"addEntity", wxr_command_addEntity};
  command_list[3] = (WXR_Command){"removeEntity", wxr_command_removeEntity};
  command_list[4] = (WXR_Command){"addComponent", wxr_command_addComponent};
  command_list[5] = (WXR_Command){"get", wxr_command_get};
  command_list[6] = (WXR_Command){"set", wxr_command_set};
  command_list[7] = (WXR_Command){"addSystem", wxr_command_addSystem};
  command_list[8] = (WXR_Command){"removeSystem", wxr_command_removeSystem};
  command_list[9] = (WXR_Command){"loadPlugin", wxr_command_loadPlugin};
  command_list[10] = (WXR_Command){"unloadPlugin", wxr_command_unloadPlugin};
  command_list[11] = (WXR_Command){"showEntities", wxr_command_showEntities};
  command_list[12] = (WXR_Command){"showPlugins", wxr_command_showPlugins};
  command_list[13] =
      (WXR_Command){"showComponents", wxr_command_showComponents};
  command_list[14] = (WXR_Command){"showSystems", wxr_command_showSystems};
  command_list[15] = (WXR_Command){"save", wxr_command_save};
  command_list[16] = (WXR_Command){"load", wxr_command_load};

  return command_list;
}

void wxr_destroy_command_list(WXR_Command *ptr) { free(ptr); }

void wxr_command_reload(char **args, WXR_Scene *scene) {
  wxr_set_scene_reload(scene);
}

void wxr_command_exit(char **args, WXR_Scene *scene) {
  wxr_set_scene_terminate(scene);
}

void wxr_command_addEntity(char **args, WXR_Scene *scene) {
  wxr_add_entity(scene);
}

void wxr_command_removeEntity(char **args, WXR_Scene *scene) {
  if (!*args) {
    wxr_warn("Remove Entity requires the entity id to remove");
    return;
  }
  size_t entity_id = strtol(args[0], NULL, 10);
  wxr_remove_entity(scene, entity_id);
}

void wxr_command_addComponent(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Add Component requires the entity id to add to the entity");
    return;
  }
  if (!args[1]) {
    wxr_warn("Add Component requires the component");
    return;
  }
  size_t entity_id = strtol(args[0], NULL, 10);
  wxr_add_component(scene, entity_id, args[1]);
}

void wxr_command_get(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Get requires the entity id to add to the entity");
    return;
  }
  if (!args[1]) {
    wxr_warn("Get requires the component");
    return;
  }
  if (!args[2]) {
    wxr_warn("Get requires the field name");
    return;
  }
  size_t entity_id = strtol(args[0], NULL, 10);
  void *component = wxr_entity_get_component(scene, entity_id, args[1]);
  if (!component) {
    wxr_warn("Component `%s` couldn't be found for entity %ld", args[1],
             entity_id);
    return;
  }
  WXR_Component_Schema *schema = wxr_get_schema_of_component(scene, component);

  WXR_Component_Field *field = wxr_get_field(schema, args[2]);
  if (!field) {
    wxr_warn("Field `%s` was not found in component `%s`", args[2], args[1]);
    return;
  }

  WXR_Primitive_Type type = wxr_get_field_type(schema, args[2]);

  WXR_Component_Getter getter = wxr_get_field_getter(schema, args[2]);

  if (!getter) {
    wxr_warn("Field `%s` has no getter function", args[2]);
    return;
  }

  const void *data = getter(component);
  wxr_assert_abort(data, "The getter of the field `%s` returned NULL", args[2]);
  if (type == WXR_L) {
    long l_data = *(long *)data;
    wxr_info("%s: %ld", args[2], l_data);
  } else if (type == WXR_F) {
    float f_data = *(float *)data;
    wxr_info("%s: %f", args[2], f_data);
  } else if (type == WXR_C) {
    char c_data = *(char *)data;
    wxr_info("%s: %c", args[2], c_data);
  } else if (type == WXR_BLOB) {
    wxr_info("%s: 0x%lx", args[2], data);
  } else if (type == WXR_S) {
    char *s_data = (char *)data;
    wxr_info("%s: %s", args[2], s_data);
  } else if (type == WXR_BLOB_ARRAY) {
    wxr_info("%s: 0x%lx (Blob Array)", args[2], data);
  } else {
    wxr_critical("WXR_Primitive_Type is not valid");
  }
}

void wxr_command_set(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Set requires the entity id to add to the entity");
    return;
  }
  if (!args[1]) {
    wxr_warn("Set requires the component");
    return;
  }
  if (!args[2]) {
    wxr_warn("Set requires the field name");
    return;
  }
  if (!args[3]) {
    wxr_warn("Set requires a value");
    return;
  }
  size_t entity_id = strtol(args[0], NULL, 10);
  void *component = wxr_entity_get_component(scene, entity_id, args[1]);
  if (!component) {
    wxr_warn("Component `%s` couldn't be found for entity %ld", args[1],
             entity_id);
    return;
  }
  WXR_Component_Schema *schema = wxr_get_schema_of_component(scene, component);

  WXR_Component_Field *field = wxr_get_field(schema, args[2]);
  if (!field) {
    wxr_warn("Field `%s` was not found in component `%s`", args[2], args[1]);
    return;
  }

  WXR_Primitive_Type type = wxr_get_field_type(schema, args[2]);

  WXR_Component_Setter setter = wxr_get_field_setter(schema, args[2]);

  if (!setter) {
    wxr_warn("Field `%s` has no setter function", args[2]);
    return;
  }

  if (type == WXR_L) {
    long l_data = strtol(args[3], NULL, 10);
    wxr_set(scene, component, args[2], &l_data);
  } else if (type == WXR_F) {
    float f_data = strtof(args[3], NULL);
    wxr_set(scene, component, args[2], &f_data);
  } else if (type == WXR_C) {
    char c_data = *args[3];
    wxr_set(scene, component, args[2], &c_data);
  } else if (type == WXR_S) {
    wxr_set(scene, component, args[2], args[3]);
  } else {
    wxr_warn("Cannot handle such a primitive type");
  }
  wxr_info("Field `%s` set", args[2]);
}

void wxr_command_addSystem(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Add System requires the system name");
    return;
  }
  int result = wxr_add_system(scene, args[0], 100);
  if (result != 0) {
    wxr_warn("Failed to add system `%s`", args[0]);
  } else {
    wxr_info("System `%s` added successfully", args[0]);
  }
}

void wxr_command_removeSystem(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Remove System requires the system name");
    return;
  }
  int result = wxr_remove_system(scene, args[0]);
  if (result != 0) {
    wxr_warn("Failed to remove system `%s`", args[0]);
  } else {
    wxr_info("System `%s` removed successfully", args[0]);
  }
}

void wxr_command_loadPlugin(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Load Plugin requires the path to a shared object file");
    return;
  }
  int result = wxr_load_plugin(scene, args[0]);
  if (result != 0) {
    wxr_warn("Failed to load plugin `%s`", args[0]);
  } else {
    wxr_info("Plugin `%s` loaded successfully", args[0]);
  }
}

void wxr_command_unloadPlugin(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Unload Plugin requires the path to a shared object file");
    return;
  }
  int result = wxr_unload_plugin(scene, args[0]);
  if (result != 0) {
    wxr_warn("Failed to unload plugin `%s`", args[0]);
  } else {
    wxr_info("Plugin `%s` unloaded successfully", args[0]);
  }
}

void wxr_command_showEntities(char **args, WXR_Scene *scene) {
  wxr_print_entities(scene);
}

void wxr_command_showPlugins(char **args, WXR_Scene *scene) {
  wxr_print_plugins(scene);
}

void wxr_command_showComponents(char **args, WXR_Scene *scene) {
  wxr_print_components(scene);
}

void wxr_command_showSystems(char **args, WXR_Scene *scene) {
  wxr_print_systems(scene);
}
void wxr_command_save(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Save requires a filename");
    return;
  }
  wxr_serialize_scene_to_file(scene, args[0]);
  wxr_info("Saved Scene");
}

void wxr_command_load(char **args, WXR_Scene *scene) {
  if (!args[0]) {
    wxr_warn("Load requires a filename");
    return;
  }

  wxr_deserialize_scene_from_file(scene, args[0]);
  wxr_info("Loaded Scene");
}
