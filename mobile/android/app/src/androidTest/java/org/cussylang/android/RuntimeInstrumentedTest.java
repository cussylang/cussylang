package org.cussylang.android;

import static org.junit.Assert.*;
import android.text.Editable;
import android.text.TextWatcher;
import android.widget.Button;
import android.widget.EditText;
import android.widget.TextView;
import androidx.test.core.app.ActivityScenario;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.junit.Test;
import org.junit.runner.RunWith;

@RunWith(AndroidJUnit4.class)
public final class RuntimeInstrumentedTest {
    @Test public void realRuntimePreservesUnicodeNulAndExitCode() throws Exception {
        CussyRuntime.Result result = CussyRuntime.run(
                "int whitecap(){jole(\"Hello λ 🎲\\0!\");verify 7;}", 0);
        assertTrue(result.diagnostic, result.ok);
        assertEquals(Long.valueOf(7), result.exitCode);
        assertEquals("Hello λ 🎲\u0000!\n", result.output);
        assertEquals("", result.diagnostic);
    }

    @Test public void fullInterpreterSupportsArraysAndPointers() throws Exception {
        CussyRuntime.Result result = CussyRuntime.run(
                "void bump(int* p){*p+=2;}int whitecap(){int a[]=[1,2];bump(&a[1]);jole(a);verify 0;}", 0);
        assertTrue(result.diagnostic, result.ok);
        assertEquals("[1, 4]\n", result.output);
    }

    @Test public void errorsKeepOutputAndDiagnostic() throws Exception {
        CussyRuntime.Result result = CussyRuntime.run(
                "int whitecap(){jole(\"before\");int a=1/0;verify a;}", 0);
        assertFalse(result.ok);
        assertNull(result.exitCode);
        assertEquals("before\n", result.output);
        assertTrue(result.diagnostic, result.diagnostic.contains("AURA_OVERFLOW"));
        CussyRuntime.Result invalid = CussyRuntime.run("int whitecap(){jole(ghost);verify 0;}", 0);
        assertFalse(invalid.ok);
        assertTrue(invalid.diagnostic, invalid.diagnostic.contains("D404"));
    }

    @Test public void executionBudgetStopsLoop() throws Exception {
        CussyRuntime.Result result = CussyRuntime.run("int whitecap(){ticker(verified){}verify 0;}", 100);
        assertFalse(result.ok);
        assertTrue(result.diagnostic, result.diagnostic.contains("LIMIT"));
    }

    @Test public void hostIoIsDisabled() throws Exception {
        CussyRuntime.Result result = CussyRuntime.run(
                "graph system;int whitecap(){jole(native1(\"\",\"cos\",0));verify 0;}", 0);
        assertFalse(result.ok);
        assertTrue(result.diagnostic, result.diagnostic.contains("CAPABILITY"));
    }

    @Test public void runButtonExecutesAndDraftSurvivesRecreation() throws Exception {
        CountDownLatch finished = new CountDownLatch(1);
        try (ActivityScenario<MainActivity> activity = ActivityScenario.launch(MainActivity.class)) {
            activity.onActivity(screen -> {
                EditText editor = screen.findViewById(R.id.source_editor);
                editor.setText("int whitecap(){jole(\"From the editor 🎲\");verify 0;}");
                TextView status = screen.findViewById(R.id.status_text);
                status.addTextChangedListener(new TextWatcher() {
                    @Override public void beforeTextChanged(CharSequence text, int start, int count, int after) {}
                    @Override public void onTextChanged(CharSequence text, int start, int before, int count) {
                        if (text.toString().startsWith("Finished")) finished.countDown();
                    }
                    @Override public void afterTextChanged(Editable text) {}
                });
                Button run = screen.findViewById(R.id.run_button);
                run.performClick();
                assertFalse(run.isEnabled());
            });
            assertTrue("Editor execution did not complete", finished.await(15, TimeUnit.SECONDS));
            activity.recreate();
            activity.onActivity(screen -> {
                EditText editor = screen.findViewById(R.id.source_editor);
                TextView output = screen.findViewById(R.id.output_text);
                Button run = screen.findViewById(R.id.run_button);
                assertTrue(editor.getText().toString().contains("From the editor 🎲"));
                assertEquals("From the editor 🎲\n", output.getText().toString());
                assertTrue(run.isEnabled());
            });
        }
    }
}
