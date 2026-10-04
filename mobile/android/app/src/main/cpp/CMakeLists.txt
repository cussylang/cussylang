cmake_minimum_required(VERSION 3.22.1)
project(cussy_android LANGUAGES C)

set(RUST_LIBRARY "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libcussy_mobile.a")
if(NOT EXISTS "${RUST_LIBRARY}")
    message(FATAL_ERROR "Missing ${RUST_LIBRARY}. Build the Android Rust runtime with the repository mobile build script first.")
endif()
add_library(cussy_mobile STATIC IMPORTED)
set_target_properties(cussy_mobile PROPERTIES IMPORTED_LOCATION "${RUST_LIBRARY}")

add_library(cussy_android SHARED cussy_jni.c)
target_compile_features(cussy_android PRIVATE c_std_11)
target_compile_options(cussy_android PRIVATE -Wall -Wextra -Werror)
target_include_directories(cussy_android PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../include")
target_link_libraries(cussy_android PRIVATE cussy_mobile android log dl m)
target_link_options(cussy_android PRIVATE
    "-Wl,-z,max-page-size=16384"
    "-Wl,-z,common-page-size=16384")
