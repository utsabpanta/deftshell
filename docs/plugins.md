# Plugin Development Guide

DeftShell supports plugins distributed through npm or installed from local directories.

## Quick Start

### Scaffold a Plugin

```bash
ds plugin create my-plugin
cd ~/.deftshell/plugins/my-plugin
```

### Plugin Structure

A plugin is a directory with a `package.json` and a JavaScript entry point:

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

```javascript
// index.js
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
        console.log('Hello from the plugin!');
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

## Plugin Management

```bash
ds plugin list              # List installed plugins
ds plugin install <name>    # Install from npm
ds plugin install ./path    # Install from local directory
ds plugin remove <name>     # Remove a plugin
ds plugin update            # Update all plugins
ds plugin search <query>    # Search npm registry
ds plugin enable <name>     # Enable a disabled plugin
ds plugin disable <name>    # Disable without removing
ds plugin info <name>       # Show plugin details
```

## Publishing

1. Ensure your `package.json` has `"keywords": ["deftshell-plugin"]`
2. Run `npm publish`
3. Users install with `ds plugin install your-plugin-name`
