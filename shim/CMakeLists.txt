cmake_minimum_required(VERSION 3.28)
project(smf_editor_host LANGUAGES C CXX)
include("${CMAKE_CURRENT_LIST_DIR}/../../cat-plugin-player/shared/host.cmake")
target_sources(uapmd_shim PRIVATE "${CMAKE_CURRENT_LIST_DIR}/project_import.cpp" "${CMAKE_CURRENT_LIST_DIR}/project_export.cpp")
target_include_directories(uapmd_shim PRIVATE
    "${UAPMD_DIR}/source/uapmd-data/include"
    "${UAPMD_DIR}/source/uapmd-data/src/project"
    "${midicci_SOURCE_DIR}/include"
    "${choc_SOURCE_DIR}")
target_link_libraries(uapmd_shim PRIVATE uapmd-data)
