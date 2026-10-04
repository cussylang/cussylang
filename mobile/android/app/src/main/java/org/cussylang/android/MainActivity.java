package org.cussylang.android;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.ContentResolver;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Typeface;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.Editable;
import android.text.InputFilter;
import android.text.InputType;
import android.text.TextWatcher;
import android.view.Gravity;
import android.view.View;
import android.view.WindowInsets;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.CharacterCodingException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public final class MainActivity extends Activity {
    private static final int OPEN_SOURCE = 1;
    private static final int SAVE_SOURCE = 2;
    private static final String EXAMPLE = "int whitecap() {\n    jole(\"Hello from Cussy!\");\n    verify 0;\n}\n";
    private Session session;
    private EditText editor;
    private Button runButton;
    private Button openButton;
    private Button saveButton;
    private TextView status;
    private TextView output;

    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Object retained = getLastNonConfigurationInstance();
        session = retained instanceof Session ? (Session) retained : new Session();
        if (retained == null) {
            session.source = preferences().getString("source", EXAMPLE);
        }
        buildUi();
        session.changed = this::render;
        render();
    }

    private SharedPreferences preferences() {
        return getSharedPreferences("editor", MODE_PRIVATE);
    }

    @Override protected void onPause() {
        // Keep large drafts out of savedInstanceState's limited Binder transaction.
        preferences().edit().putString("source", session.source).apply();
        super.onPause();
    }

    @Override public Object onRetainNonConfigurationInstance() {
        return session;
    }

    @Override protected void onDestroy() {
        session.changed = null;
        if (!isChangingConfigurations()) session.executor.shutdown();
        super.onDestroy();
    }

    private void buildUi() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        int spacing = dp(16);
        root.setPadding(spacing, spacing, spacing, spacing);
        root.setOnApplyWindowInsetsListener((view, insets) -> {
            // Also includes the keyboard when adjustResize dispatches IME insets.
            if (android.os.Build.VERSION.SDK_INT >= 30) {
                android.graphics.Insets safe = insets.getInsets(
                        WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout()
                                | WindowInsets.Type.ime());
                view.setPadding(spacing + safe.left, spacing + safe.top,
                        spacing + safe.right, spacing + safe.bottom);
            } else {
                view.setPadding(spacing + insets.getSystemWindowInsetLeft(),
                        spacing + insets.getSystemWindowInsetTop(),
                        spacing + insets.getSystemWindowInsetRight(),
                        spacing + insets.getSystemWindowInsetBottom());
            }
            return insets;
        });
        TextView title = label("Cussy", 26);
        title.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        root.addView(title);
        TextView note = label(getString(R.string.runtime_note), 13);
        note.setPadding(0, dp(4), 0, dp(12));
        root.addView(note);
        TextView sourceLabel = label(getString(R.string.source), 14);
        sourceLabel.setLabelFor(R.id.source_editor);
        root.addView(sourceLabel);

        editor = new EditText(this);
        editor.setId(R.id.source_editor);
        editor.setGravity(Gravity.TOP | Gravity.START);
        editor.setTypeface(Typeface.MONOSPACE);
        editor.setTextSize(15);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE
                | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        editor.setHorizontallyScrolling(true);
        editor.setFilters(new InputFilter[]{new InputFilter.LengthFilter(SourceFiles.MAX_BYTES)});
        editor.setSaveEnabled(false);
        editor.setImportantForAutofill(View.IMPORTANT_FOR_AUTOFILL_NO);
        editor.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence text, int start, int count, int after) {}
            @Override public void onTextChanged(CharSequence text, int start, int before, int count) {
                session.source = text.toString();
            }
            @Override public void afterTextChanged(Editable text) {}
        });
        root.addView(editor, new LinearLayout.LayoutParams(-1, 0, 3));

        LinearLayout actions = new LinearLayout(this);
        openButton = new Button(this);
        openButton.setId(R.id.open_button);
        openButton.setText(R.string.open);
        openButton.setOnClickListener(view -> openSource());
        actions.addView(openButton, new LinearLayout.LayoutParams(0, -2, 1));
        saveButton = new Button(this);
        saveButton.setId(R.id.save_button);
        saveButton.setText(R.string.save);
        saveButton.setOnClickListener(view -> saveSource());
        actions.addView(saveButton, new LinearLayout.LayoutParams(0, -2, 1));
        runButton = new Button(this);
        runButton.setId(R.id.run_button);
        runButton.setText(R.string.run);
        runButton.setOnClickListener(view -> session.run());
        actions.addView(runButton, new LinearLayout.LayoutParams(0, -2, 1));
        root.addView(actions);
        status = label(getString(R.string.ready), 14);
        status.setId(R.id.status_text);
        status.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        root.addView(status);
        TextView outputLabel = label(getString(R.string.output), 14);
        outputLabel.setPadding(0, dp(12), 0, dp(4));
        outputLabel.setLabelFor(R.id.output_text);
        root.addView(outputLabel);
        ScrollView scroll = new ScrollView(this);
        output = label("", 14);
        output.setId(R.id.output_text);
        output.setTypeface(Typeface.MONOSPACE);
        output.setTextIsSelectable(true);
        output.setSaveEnabled(false);
        scroll.addView(output);
        root.addView(scroll, new LinearLayout.LayoutParams(-1, 0, 2));
        setContentView(root);
        root.requestApplyInsets();
    }

    private TextView label(String text, int size) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(size);
        return view;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private void render() {
        if (!editor.getText().toString().equals(session.source)) editor.setText(session.source);
        editor.setEnabled(!session.busy);
        runButton.setEnabled(!session.busy);
        openButton.setEnabled(!session.busy);
        saveButton.setEnabled(!session.busy);
        status.setText(session.status);
        output.setText(session.output);
    }

    private void openSource() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        // .cussy has no registered MIME type; providers may call it octet-stream.
        intent.setType("*/*");
        try {
            startActivityForResult(intent, OPEN_SOURCE);
        } catch (ActivityNotFoundException error) {
            session.status = "This device has no document picker.";
            render();
        }
    }

    private void saveSource() {
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("text/plain");
        intent.putExtra(Intent.EXTRA_TITLE, "program.cussy");
        session.pendingExport = session.source;
        try {
            startActivityForResult(intent, SAVE_SOURCE);
        } catch (ActivityNotFoundException error) {
            session.status = "This device has no document picker.";
            render();
        }
    }

    @Override protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == OPEN_SOURCE && resultCode == RESULT_OK && data != null) {
            Uri uri = data.getData();
            if (uri != null) session.open(getApplicationContext().getContentResolver(), uri);
        } else if (requestCode == SAVE_SOURCE && resultCode == RESULT_OK && data != null) {
            Uri uri = data.getData();
            if (uri != null) session.save(getApplicationContext().getContentResolver(), uri);
        }
    }

    // Retained across rotations: one interpreter job, no Activity retained by the worker.
    private static final class Session {
        final ExecutorService executor = Executors.newSingleThreadExecutor();
        final Handler main = new Handler(Looper.getMainLooper());
        String source = EXAMPLE;
        String status = "Ready";
        String output = "";
        String pendingExport;
        boolean busy;
        Runnable changed;

        void notifyUi() { if (changed != null) changed.run(); }

        void run() {
            if (busy) return;
            String program = source;
            busy = true;
            status = "Running…";
            output = "";
            notifyUi();
            executor.execute(() -> {
                try {
                    CussyRuntime.Result result = CussyRuntime.run(program, CussyRuntime.DEFAULT_FUEL);
                    main.post(() -> {
                        output = displayOutput(result.output, result.diagnostic);
                        status = result.ok ? "Finished · exit " + result.exitCode : "Program error";
                        busy = false;
                        notifyUi();
                    });
                } catch (IOException | RuntimeException | LinkageError error) {
                    failed(error);
                }
            });
        }

        void open(ContentResolver resolver, Uri uri) {
            if (busy) return;
            busy = true;
            status = "Opening…";
            notifyUi();
            executor.execute(() -> {
                try (InputStream input = resolver.openInputStream(uri)) {
                    String imported = SourceFiles.read(input);
                    main.post(() -> {
                        source = imported;
                        output = "";
                        status = "Opened · ready to run";
                        busy = false;
                        notifyUi();
                    });
                } catch (IOException | RuntimeException error) {
                    failed(error);
                }
            });
        }

        void save(ContentResolver resolver, Uri uri) {
            if (busy) return;
            String program = pendingExport == null ? source : pendingExport;
            pendingExport = null;
            busy = true;
            status = "Saving…";
            notifyUi();
            executor.execute(() -> {
                try {
                    // Validate before opening the destination for writing.
                    byte[] bytes = SourceFiles.encode(program);
                    try (OutputStream destination = resolver.openOutputStream(uri, "wt")) {
                        if (destination == null) throw new IOException("The file provider cannot save this file.");
                        destination.write(bytes);
                    }
                    main.post(() -> {
                        status = "Saved .cussy file";
                        busy = false;
                        notifyUi();
                    });
                } catch (IOException | RuntimeException error) {
                    failed(error);
                }
            });
        }

        void failed(Throwable error) {
            String message = error instanceof CharacterCodingException
                    ? "The source must contain valid UTF-8 text."
                    : error.getMessage();
            String diagnostic = message == null ? error.getClass().getSimpleName() : message;
            main.post(() -> {
                status = "Could not complete the operation";
                output = diagnostic;
                busy = false;
                notifyUi();
            });
        }

        static String displayOutput(String text, String diagnostic) {
            // Bound TextView work even when a program reaches the runtime output limit.
            int limit = 65_536;
            if (text.length() > limit) {
                int end = Character.isHighSurrogate(text.charAt(limit - 1)) ? limit - 1 : limit;
                text = text.substring(0, end) + "\n[Display truncated after 65,536 characters]\n";
            }
            if (!diagnostic.isEmpty()) {
                text += (text.isEmpty() || text.endsWith("\n") ? "" : "\n") + diagnostic;
            }
            return text;
        }
    }
}
