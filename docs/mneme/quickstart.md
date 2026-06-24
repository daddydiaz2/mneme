# Quick Start

## 1. Save Your First Memory

```bash
mneme save --project my-app \
  --title "JWT auth middleware" \
  --type decision \
  --importance high \
  --tags rust,auth \
  "We chose JWT Bearer tokens for stateless auth across instances"
```

## 2. Search

```bash
mneme search "JWT auth" --project my-app
```

## 3. Configure Your Agent

```bash
mneme setup opencode
mneme setup claude-code
mneme setup cursor
```

## 4. Use MCP Tools from Your Agent

Once configured, your agent can call mneme tools automatically. Example from an AI coding agent:

```
mem_save(project: "my-app", title: "Fixed N+1 in UserList", 
         type: "bugfix", importance: "high",
         content: "Added eager loading to User queries")
```

## 5. View Your Knowledge Graph

```bash
mneme tui
# Press Tab to see the graph view
```
