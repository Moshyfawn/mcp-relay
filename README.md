# MCP Relay

Minimal binary that bridges stdio-based MCP clients to remote MCP servers over HTTP.

```
MCP Client (stdio) ←→ mcp-relay ←→ Remote MCP Server (HTTP/SSE)
```

## Usage

```bash
mcp-relay <server-url>
```

## Protocol

Implements MCP 2025-03-26 Streamable HTTP transport:

- Reads JSON-RPC from stdin (newline-delimited)
- POSTs to server, handles JSON and SSE responses
- Writes responses to stdout
- Manages `Mcp-Session-Id` header

## Build

```bash
cargo build --release
```
