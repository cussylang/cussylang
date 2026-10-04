package org.cussylang.android;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.CharBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;

final class SourceFiles {
    static final int MAX_BYTES = 1_048_576;

    private SourceFiles() {}

    static String read(InputStream input) throws IOException {
        if (input == null) throw new IOException("The file provider returned no data.");
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        int length;
        while ((length = input.read(buffer)) != -1) {
            if (length > MAX_BYTES - bytes.size()) {
                throw new IOException("Source files must be at most 1 MiB.");
            }
            bytes.write(buffer, 0, length);
        }
        String source = decode(bytes.toByteArray());
        // UTF-8 editors sometimes include a BOM; it is not program text.
        return source.startsWith("\uFEFF") ? source.substring(1) : source;
    }

    static String decode(byte[] bytes) throws CharacterCodingException {
        return StandardCharsets.UTF_8.newDecoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .decode(ByteBuffer.wrap(bytes)).toString();
    }

    static byte[] encode(String source) throws IOException {
        ByteBuffer buffer = StandardCharsets.UTF_8.newEncoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .encode(CharBuffer.wrap(source));
        if (buffer.remaining() > MAX_BYTES) {
            throw new IOException("Source files must be at most 1 MiB.");
        }
        byte[] bytes = new byte[buffer.remaining()];
        buffer.get(bytes);
        return bytes;
    }
}
