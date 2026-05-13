cmake_minimum_required(VERSION 3.10)
set(CMAKE_C_COMPILER clang)
project(WasserXR LANGUAGES C)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CLANG_TIDY clang-tidy)

option(BUILD_DEBUG "Debug build" OFF)
option(WXR_STATIC "Static Linking for Components and Systems" OFF)
option(WXR_TESTS "Build the Tests for WasserXR" ON)

add_compile_options(-std=c99 -Wall -Wextra -Wno-unused-parameter)
add_link_options(-rdynamic)

if (BUILD_DEBUG)
  add_compile_options(-O0 -g -fno-omit-frame-pointer)
  add_compile_options(-fsanitize=address -fsanitize=undefined)
  add_link_options(-fsanitize=address -fsanitize=undefined)
else()
  add_compile_options(-O3)
endif()

if (WXR_STATIC)
  add_compile_definitions(WXR_STATIC)
endif()

find_package(PkgConfig REQUIRED)
pkg_check_modules(GLIB REQUIRED glib-2.0)
pkg_check_modules(CGLM REQUIRED cglm)
pkg_check_modules(ASSIMP REQUIRED assimp)
pkg_check_modules(WASSERXR_ECS REQUIRED wasserxr_ecs)

# Core (merged Components and Systems with internal Shader, Mesh, Mesh_Data utilities)
file(GLOB CORE_SRC src/WasserXR/core/*.c src/WasserXR/core/internal/*.c)
if(WXR_STATIC)
  add_library(wasserxr_core STATIC ${CORE_SRC})
else()
  add_library(wasserxr_core SHARED ${CORE_SRC})
endif()
target_include_directories(wasserxr_core PUBLIC include)
target_include_directories(wasserxr_core PRIVATE src/WasserXR/core/internal)
target_include_directories(wasserxr_core PRIVATE ${GLIB_INCLUDE_DIRS} ${CGLM_INCLUDE_DIRS} ${ASSIMP_INCLUDE_DIRS} ${WASSERXR_ECS_INCLUDE_DIRS})
target_link_libraries(wasserxr_core PRIVATE glfw pthread ${CGLM_LIBRARIES} m ${GLIB_LIBRARIES} ${ASSIMP_LIBRARIES} ${WASSERXR_ECS_LIBRARIES})

# ===
# Install
# ===

include(GNUInstallDirs)

install(TARGETS wasserxr_core
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
)

# Install include directory
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

# ===
# Tests
# ===

# Tests
enable_testing()

# Helper function to simplify adding tests
# Usage: add_wasserxr_test(test_name)
# Assumes test file is located at tests/{test_name}.c
function(add_wasserxr_test test_name)
    add_executable(${test_name} tests/${test_name}.c src/glad/gl.c)
    target_include_directories(${test_name} PRIVATE ${GLIB_INCLUDE_DIRS})
    target_link_libraries(${test_name} PRIVATE wasserxr_ecs ${GLIB_LIBRARIES})
    add_test(NAME ${test_name} COMMAND ${test_name})
endfunction()

if(WXR_TESTS)
endif()
