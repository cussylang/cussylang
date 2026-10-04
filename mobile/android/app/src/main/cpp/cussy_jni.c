#include <jni.h>
#include <limits.h>
#include <stdint.h>
#include <string.h>
#include "cussy_mobile.h"

static void fail(JNIEnv *env, const char *message) {
    jclass type = (*env)->FindClass(env, "java/lang/IllegalStateException");
    if (type != NULL) {
        (*env)->ThrowNew(env, type, message);
    }
}

JNIEXPORT jbyteArray JNICALL
Java_org_cussylang_android_CussyRuntime_nativeRun(JNIEnv *env, jclass type,
                                                jbyteArray source, jlong fuel) {
    (void)type;
    if (source == NULL || fuel < 0) {
        fail(env, "Invalid runtime input");
        return NULL;
    }
    jsize length = (*env)->GetArrayLength(env, source);
    jbyte *bytes = (*env)->GetByteArrayElements(env, source, NULL);
    if (bytes == NULL) {
        return NULL; /* A pending Java allocation exception is preserved. */
    }
    char *json = cussy_mobile_run((const uint8_t *)bytes, (size_t)length, (uint64_t)fuel);
    (*env)->ReleaseByteArrayElements(env, source, bytes, JNI_ABORT);
    if (json == NULL) {
        fail(env, "The runtime could not allocate a result");
        return NULL;
    }
    size_t size = strlen(json);
    if (size > INT_MAX) {
        cussy_mobile_free(json);
        fail(env, "The runtime result is too large");
        return NULL;
    }
    jbyteArray result = (*env)->NewByteArray(env, (jsize)size);
    if (result != NULL) {
        (*env)->SetByteArrayRegion(env, result, 0, (jsize)size, (const jbyte *)json);
    }
    cussy_mobile_free(json);
    return result;
}
