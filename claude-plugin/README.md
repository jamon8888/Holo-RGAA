# `claude-plugin/` is deprecated — use `rgaa-rs/plugins/rgaa-consultant/`

This directory used to hold a second, older copy of the Claude Code plugin
(manifest name `rgaa-audit`, version 0.1.0). The canonical plugin is:

```
rgaa-rs/plugins/rgaa-consultant/
```

Everything that lived only here — the four agents, `hooks/hooks.json`,
`scripts/check-runtime.sh`, the `verify` skill and the `tests/` scripts — moved
there. The duplicated skills, `.mcp.json` and `.claude-plugin/plugin.json` are
gone: they were the stale fork, and keeping a second manifest meant installers
and CI could each pick a different plugin (they did — `install.sh` and
`docs/rgaa-plugin-install.md` installed this one).

This directory is kept as a pointer only, so that a bookmark, an old shell
history line or a script that still names `claude-plugin/` lands on this note
instead of a 404. It is not a plugin: there is no manifest here, so Claude Code
will not load it.

## If you installed from this directory

An existing install under `~/.claude/plugins/rgaa-audit` still works but is
frozen at the old content. Replace it:

```bash
rm -rf ~/.claude/plugins/rgaa-audit
ln -s "$(pwd)/rgaa-rs/plugins/rgaa-consultant" ~/.claude/plugins/rgaa-accessibility
```

Or re-run `./install.sh`, which now installs the canonical tree (and removes the
stale `rgaa-audit` directory for you).

See `rgaa-rs/plugins/rgaa-consultant/README.md` for the plugin documentation and
`docs/rgaa-plugin-install.md` for the installation guide.
