package org.cussylang.android;

import static org.junit.Assert.*;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.StandardCharsets;
import org.junit.Test;

public final class SourceFilesTest {
    @Test public void unicodeAndEmbeddedNulRoundTrip() throws Exception {
        String source = "λ 🎲\u0000";
        assertEquals(source, SourceFiles.read(new ByteArrayInputStream(SourceFiles.encode(source))));
    }

    @Test public void stripsOnlyLeadingUtf8Bom() throws Exception {
        assertEquals("a\uFEFF", SourceFiles.read(new ByteArrayInputStream(
                "\uFEFFa\uFEFF".getBytes(StandardCharsets.UTF_8))));
    }

    @Test public void invalidUtf8AndUnpairedSurrogatesAreRejected() {
        assertThrows(CharacterCodingException.class,
                () -> SourceFiles.read(new ByteArrayInputStream(new byte[]{(byte) 0xc3, 0x28})));
        assertThrows(CharacterCodingException.class, () -> SourceFiles.encode("\uD800"));
    }

    @Test public void importLimitCountsBytesAndAcceptsExactBoundary() throws Exception {
        byte[] boundary = new byte[SourceFiles.MAX_BYTES];
        java.util.Arrays.fill(boundary, (byte) 'a');
        assertEquals(SourceFiles.MAX_BYTES,
                SourceFiles.read(new ByteArrayInputStream(boundary)).length());
        assertThrows(IOException.class, () -> SourceFiles.read(
                new ByteArrayInputStream(new byte[SourceFiles.MAX_BYTES + 1])));
        String multibyte = "λ".repeat(SourceFiles.MAX_BYTES / 2 + 1);
        assertThrows(IOException.class, () -> SourceFiles.encode(multibyte));
    }
}
