/* Compile against libcussy_mobile to verify the public header and C ABI. */
#include "cussy_mobile.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    const char source[] = "graph math; int whitecap(){jole(\"mobile C ABI\",sqrt(9));verify 0;}";
    assert(cussy_mobile_api_version() == 1);
    char *result = cussy_mobile_run((const uint8_t *)source, strlen(source), 0);
    assert(result != NULL);
    assert(strstr(result, "\"ok\":true") != NULL);
    assert(strstr(result, "\"output\":\"mobile C ABI 3\\n\"") != NULL);
    assert(strstr(result, "\"exit_code\":0") != NULL);
    cussy_mobile_free(result);
    result = cussy_mobile_run(NULL, 1, 0);
    assert(result != NULL && strstr(result, "\"ok\":false") != NULL);
    cussy_mobile_free(result);
    cussy_mobile_free(NULL);
    puts("Cussy mobile C ABI smoke passed");
    return 0;
}
