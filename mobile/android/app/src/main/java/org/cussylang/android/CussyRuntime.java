package org.cussylang.android;

import java.io.IOException;
import org.json.JSONException;
import org.json.JSONObject;

public final class CussyRuntime {
    public static final long DEFAULT_FUEL = 5_000_000;

    static { System.loadLibrary("cussy_android"); }

    private CussyRuntime() {}

    public static final class Result {
        public final boolean ok;
        public final Long exitCode;
        public final String output;
        public final String diagnostic;

        private Result(JSONObject json) throws JSONException {
            ok = json.getBoolean("ok");
            exitCode = json.isNull("exit_code") ? null : json.getLong("exit_code");
            output = json.getString("output");
            diagnostic = json.getString("diagnostic");
        }
    }

    public static Result run(String source, long fuel) throws IOException {
        if (fuel < 0) throw new IllegalArgumentException("Fuel cannot be negative.");
        byte[] json = nativeRun(SourceFiles.encode(source), fuel);
        try {
            return new Result(new JSONObject(SourceFiles.decode(json)));
        } catch (JSONException error) {
            throw new IOException("The runtime returned an invalid result.", error);
        }
    }

    // Ordinary UTF-8 bytes, never JNI modified UTF-8 (which changes emoji and NUL).
    private static native byte[] nativeRun(byte[] source, long fuel);
}
