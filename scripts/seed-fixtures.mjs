// Development fixtures only. Never reads or modifies a real agent configuration.
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
const root=resolve('.local-dev/fixture-home');
const files={
  '.codex/config.toml':'# MCP Deck isolated fixture\nmodel = "keep-existing-model"\n\n[mcp_servers.filesystem]\ncommand = "npx"\nargs = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]\n\n[mcp_servers.context7]\nurl = "https://mcp.context7.com/mcp"\n',
  '.claude.json':JSON.stringify({preferences:{keep:true},mcpServers:{filesystem:{command:'npx',args:['-y','@modelcontextprotocol/server-filesystem','/workspace']},github:{type:'http',url:'https://api.githubcopilot.com/mcp/'},fetch:{command:'uvx',args:['mcp-server-fetch']}}},null,2),
  '.cursor/mcp.json':JSON.stringify({mcpServers:{context7:{url:'https://mcp.context7.com/mcp'}}},null,2),
  '.gemini/settings.json':JSON.stringify({theme:'keep-me',mcpServers:{docs:{httpUrl:'https://example.com/mcp'}}},null,2),
  '.config/opencode/opencode.jsonc':'{\n  // Preserve custom settings\n  "theme": "system",\n  "mcp": {"memory":{"type":"local","command":["npx","-y","@modelcontextprotocol/server-memory"],"enabled":false}}\n}',
};
for(const [file,text] of Object.entries(files)){const path=resolve(root,file);await mkdir(dirname(path),{recursive:true});try{await writeFile(path,text,{flag:'wx',mode:0o600});}catch(e){if(e.code!=='EEXIST')throw e;}}
console.log(`Fixture configurations ready: ${root}`);
