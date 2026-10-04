# Cussy VS Code starter

Open this directory in VS Code and press F5 to start an Extension Development
Host. Open a `.cussy` file there (`.csy` remains supported for compatibility). This declarative extension supplies
TextMate highlighting, comments/bracket configuration, and snippets. It has no
Node dependencies, activation code, network requests, or language server.

Alternatively copy this folder into your local VS Code extensions directory as
`cussy-language-0.2.0`, then reload VS Code. Manual local use does not publish
anything. The placeholder publisher is not a registered Marketplace identity.
For distribution, choose your publisher and package with your normal VS Code
extension tools; no Marketplace upload has been performed.

Example task to add to your program workspace's `.vscode/tasks.json` after
installing `cussy` on PATH:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Cussy: check",
      "type": "process",
      "command": "cussy",
      "args": ["check", "${file}"],
      "problemMatcher": []
    },
    {
      "label": "Cussy: run",
      "type": "process",
      "command": "cussy",
      "args": ["run", "${file}", "--brainrot"],
      "problemMatcher": []
    }
  ]
}
```

For formatting, use `cussy fmt file.cussy`. Native Format Document, completions,
hover types, go-to-definition, debugging, and squiggle diagnostics require a future
language server and are not claimed by this starter. The TextMate grammar is a
lexical approximation; the Cussy parser remains authoritative for contextual words.
