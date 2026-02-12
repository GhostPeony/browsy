# Framework Integrations

browsy provides native integrations for popular AI/agent frameworks in both Python and JavaScript/TypeScript. Each integration wraps browsy as framework-compatible tools, so agents can browse the web using their native tool-calling patterns.

## JavaScript / TypeScript

The `browsy-ai` npm package provides integrations for LangChain.js, OpenAI, and Vercel AI SDK. Install the core package and whichever framework you use:

```bash
npm install browsy-ai                    # Core SDK
npm install browsy-ai @langchain/core    # + LangChain.js
npm install browsy-ai openai             # + OpenAI
npm install browsy-ai ai                 # + Vercel AI SDK
```

### LangChain.js

```typescript
import { getTools } from "browsy-ai/langchain";

const tools = getTools();  // -> 14 LangChain tool instances
```

### OpenAI function calling

```typescript
import { getToolDefinitions, handleToolCall } from "browsy-ai/openai";

const tools = getToolDefinitions();
const result = await handleToolCall("browsy_browse", { url: "https://example.com" });
```

### Vercel AI SDK

```typescript
import { browsyTools } from "browsy-ai/vercel-ai";
import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";

const result = await generateText({
  model: openai("gpt-4o"),
  tools: browsyTools(),
  prompt: "Go to example.com and summarize it",
  maxSteps: 10,
});
```

See the full [JavaScript / TypeScript guide](./javascript.md) for complete examples and API reference.

---

## Python

Install browsy with the extras for your framework:

```bash
pip install browsy[langchain]   # LangChain tools
pip install browsy[crewai]      # CrewAI tool
pip install browsy[openai]      # OpenAI function calling
pip install browsy[autogen]     # AutoGen integration
pip install browsy[smolagents]  # HuggingFace smolagents
pip install browsy[all]         # All integrations
```

All Python integrations share a lazily-initialized `Browser` instance. You can pass your own `Browser` for custom viewport configuration.

## LangChain

The LangChain integration provides individual tools that plug directly into LangChain agents and chains.

```python
from browsy.langchain import get_tools
```

### Available tools

| Tool class | Description |
|------------|-------------|
| `BrowsyBrowseTool` | Navigate to a URL, returns Spatial DOM |
| `BrowsyClickTool` | Click an element by ID |
| `BrowsyTypeTextTool` | Type text into an input field |
| `BrowsySearchTool` | Web search via DuckDuckGo or Google |
| `BrowsyLoginTool` | Fill and submit a login form |
| `BrowsyPageInfoTool` | Get page metadata and suggested actions |

### Quick start

```python
from browsy.langchain import get_tools
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent

llm = ChatOpenAI(model="gpt-4o")
tools = get_tools()

agent = create_react_agent(llm, tools)

result = agent.invoke({
    "messages": [{"role": "user", "content": "Go to news.ycombinator.com and list the top 5 stories"}]
})
```

### Custom browser

Pass a `Browser` instance to control viewport size or other settings:

```python
from browsy import Browser
from browsy.langchain import get_tools

browser = Browser(viewport_width=375, viewport_height=812)
tools = get_tools(browser=browser)
```

### Using individual tools

```python
from browsy.langchain import BrowsyBrowseTool, BrowsyClickTool

browse = BrowsyBrowseTool()
page = browse.invoke({"url": "https://example.com"})

click = BrowsyClickTool()
result = click.invoke({"id": 3})
```

## CrewAI

The CrewAI integration wraps all browsy actions into a single tool that CrewAI agents can call.

```python
from browsy.crewai import BrowsyTool
```

### Quick start

```python
from browsy.crewai import BrowsyTool
from crewai import Agent, Task, Crew

browsy_tool = BrowsyTool()

researcher = Agent(
    role="Web Researcher",
    goal="Find and summarize information from web pages",
    backstory="You are an expert at navigating websites and extracting key information.",
    tools=[browsy_tool],
    verbose=True,
)

task = Task(
    description="Go to https://news.ycombinator.com and summarize the top 3 stories.",
    expected_output="A summary of the top 3 Hacker News stories with titles and URLs.",
    agent=researcher,
)

crew = Crew(agents=[researcher], tasks=[task])
result = crew.kickoff()
print(result)
```

### Tool actions

The `BrowsyTool` accepts a JSON string with an `action` field and action-specific parameters:

```python
# Browse
browsy_tool.run('{"action": "browse", "url": "https://example.com"}')

# Click
browsy_tool.run('{"action": "click", "id": 3}')

# Type
browsy_tool.run('{"action": "type", "id": 5, "text": "hello"}')

# Search
browsy_tool.run('{"action": "search", "query": "rust web framework"}')

# Login
browsy_tool.run('{"action": "login", "username": "user@example.com", "password": "secret"}')

# Page info
browsy_tool.run('{"action": "page_info"}')
```

## OpenAI function calling

The OpenAI integration provides tool definitions compatible with the OpenAI Chat Completions API and a dispatcher to handle tool calls.

```python
from browsy.openai import get_tool_definitions, handle_tool_call
```

### Tool definitions

`get_tool_definitions()` returns a list of OpenAI-compatible tool schemas:

```python
from browsy.openai import get_tool_definitions

tools = get_tool_definitions()
# Returns list of {"type": "function", "function": {"name": ..., "parameters": ...}}
```

### Handling tool calls

`handle_tool_call(name, args)` dispatches a tool call to browsy and returns the result as a string:

```python
from browsy.openai import handle_tool_call

result = handle_tool_call("browsy_browse", {"url": "https://example.com"})
```

### Complete example

```python
import json
from openai import OpenAI
from browsy.openai import get_tool_definitions, handle_tool_call

client = OpenAI()
tools = get_tool_definitions()

messages = [
    {"role": "user", "content": "Go to example.com and tell me what's on the page."}
]

# Initial request
response = client.chat.completions.create(
    model="gpt-4o",
    messages=messages,
    tools=tools,
)

# Tool call loop
while response.choices[0].message.tool_calls:
    msg = response.choices[0].message
    messages.append(msg)

    for tool_call in msg.tool_calls:
        args = json.loads(tool_call.function.arguments)
        result = handle_tool_call(tool_call.function.name, args)

        messages.append({
            "role": "tool",
            "tool_call_id": tool_call.id,
            "content": result,
        })

    response = client.chat.completions.create(
        model="gpt-4o",
        messages=messages,
        tools=tools,
    )

print(response.choices[0].message.content)
```

### Available functions

| Function name | Parameters | Description |
|---------------|------------|-------------|
| `browsy_browse` | `url`, `format?`, `scope?` | Navigate to a URL |
| `browsy_click` | `id` | Click an element |
| `browsy_type_text` | `id`, `text` | Type into an input |
| `browsy_search` | `query`, `engine?` | Web search |
| `browsy_login` | `username`, `password` | Login to a site |
| `browsy_page_info` | (none) | Get page metadata |

## AutoGen

The AutoGen integration provides a `BrowsyBrowser` class compatible with Microsoft AutoGen's `ConversableAgent`.

```python
from browsy.autogen import BrowsyBrowser
```

### Quick start

```python
from browsy.autogen import BrowsyBrowser
from autogen import ConversableAgent, UserProxyAgent

browser = BrowsyBrowser()

assistant = ConversableAgent(
    name="web_assistant",
    system_message="You help users browse the web and extract information.",
    llm_config={"config_list": [{"model": "gpt-4o"}]},
)

# Register browsy tools with the agent
browser.register(assistant)

user = UserProxyAgent(
    name="user",
    human_input_mode="NEVER",
    code_execution_config=False,
)
browser.register(user)

user.initiate_chat(
    assistant,
    message="Go to https://example.com and describe what you see.",
)
```

### Custom browser

```python
from browsy import Browser
from browsy.autogen import BrowsyBrowser

custom = Browser(viewport_width=1366, viewport_height=768)
browser = BrowsyBrowser(browser=custom)
```

## Smolagents

The smolagents integration provides a tool compatible with HuggingFace's [smolagents](https://github.com/huggingface/smolagents) framework.

```python
from browsy.smolagents import BrowsyTool
```

### Quick start

```python
from browsy.smolagents import BrowsyTool
from smolagents import CodeAgent, HfApiModel

tool = BrowsyTool()

agent = CodeAgent(
    tools=[tool],
    model=HfApiModel("Qwen/Qwen2.5-Coder-32B-Instruct"),
)

result = agent.run("Go to https://example.com and extract the main heading text.")
print(result)
```

### Custom browser

```python
from browsy import Browser
from browsy.smolagents import BrowsyTool

browser = Browser(viewport_width=1920, viewport_height=1080)
tool = BrowsyTool(browser=browser)
```

## OpenClaw / SimpleClaw

The `@openclaw/browsy` plugin integrates browsy as a first-class tool in [OpenClaw](https://openclaw.dev) and compatible frameworks like [SimpleClaw](https://simpleclaw.dev). Unlike the Python integrations above, this is a TypeScript/Node.js plugin that manages its own browsy server process.

```bash
npm install @openclaw/browsy
```

```typescript
import { register } from "@openclaw/browsy";
export default { register };
```

The plugin auto-starts a `browsy serve` process and injects 14 browsing tools into every agent. It can also intercept built-in Playwright browser tools for a transparent speed upgrade.

See the full [OpenClaw / SimpleClaw integration guide](./openclaw.md) for configuration, standalone usage, and custom orchestrator support.

## Shared Browser instance

All integrations lazily initialize a `Browser` instance with default settings (1920x1080 viewport) if none is provided. The `Browser` instance is shared across all tool calls within the same integration, maintaining session state (cookies, history, form values) across interactions.

To share a single `Browser` across multiple integrations:

```python
from browsy import Browser
from browsy.langchain import get_tools as get_langchain_tools
from browsy.openai import get_tool_definitions

browser = Browser(viewport_width=1920, viewport_height=1080)

# Both use the same session
langchain_tools = get_langchain_tools(browser=browser)
openai_tools = get_tool_definitions(browser=browser)
```
