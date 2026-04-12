# Plugin Development

DeftShell supports plugins distributed through npm or installed from local directories.

## Quick Start

### Scaffold a Plugin

```bash
ds plugin create my-plugin
cd ~/.deftshell/plugins/my-plugin
```

## Plugin Structure

A plugin is a directory with a `package.json` (or `plugin.toml`) and a JavaScript entry point.

### package.json

```json
{
  "name": "deftshell-my-plugin",
  "version": "0.1.0",
  "main": "index.js",
  "keywords": ["deftshell-plugin"],
  "deftshell": {
    "type": "command"
  }
}
```

Valid plugin types: `stack-pack`, `ai-provider`, `theme`, `command`, `safety-rule`, `integration`.

### index.js

```javascript
module.exports = {
  name: 'deftshell-my-plugin',
  version: '0.1.0',
  type: 'command',
  description: 'My awesome DeftShell plugin',

  async onActivate(context) {
    console.log('Plugin activated');
  },

  async onDeactivate() {
    // Cleanup resources
  },

  commands: [
    {
      name: 'greet',
      description: 'Say hello',
      async handler(args, context) {
        console.log(`Hello from the plugin!`);
      },
    },
  ],

  safetyRules: [
    {
      name: 'no-delete-namespace',
      pattern: 'kubectl delete namespace',
      level: 'critical',
      message: 'Deleting a Kubernetes namespace is irreversible',
    },
  ],
};
```

## Plugin Capabilities

### Commands

Add custom commands accessible via `ds <command>`:

```javascript
commands: [{
  name: 'deploy',
  description: 'Deploy to production',
  async handler(args, context) {
    const env = args[0] || 'staging';
    // Your deployment logic
  }
}]
```

### Safety Rules

Add project-specific safety rules:

```javascript
safetyRules: [{
  name: 'no-delete-namespace',
  pattern: 'kubectl delete namespace',
  level: 'critical',
  message: 'Deleting a Kubernetes namespace is irreversible',
}]
```

## How Plugins Work

**Plugin Loader**: Scans `~/.deftshell/plugins/` for directories containing `package.json` or `plugin.toml`. A `.disabled` marker file controls enable/disable without removing.

**Plugin Runtime**: Plugins run as Node.js subprocesses. DeftShell invokes `node <entry_point> <command> <args>` and captures stdout.

**Plugin Manifest** supports two formats:
- `package.json` with `"keywords": ["deftshell-plugin"]` and a `"deftshell": { "type": "..." }` field
- `plugin.toml` with plugin metadata fields

## Publishing

1. Ensure your `package.json` has `"keywords": ["deftshell-plugin"]`
2. Run `npm publish`
3. Users install with `ds plugin install your-plugin-name`
