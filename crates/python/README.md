# browsy-ai

Zero-render browser SDK for AI agents. Browse, interact with, and extract data from web pages without launching a browser.

browsy converts HTML into a **Spatial DOM** — a flat list of interactive elements with bounding boxes, roles, and states — at a fraction of the cost and latency of screenshot-based automation.

| | Screenshot-based | browsy |
|---|---|---|
| **Runtime** | Chromium process | None (Rust library) |
| **Memory** | ~300MB/page | ~5MB/page |
| **Latency** | 2-5s | <100ms |
| **Token cost** | ~10k+ | ~200-800 |

## Install

```bash
pip install browsy-ai
```

With framework integrations:

```bash
pip install browsy-ai[langchain]   # LangChain tools
pip install browsy-ai[crewai]      # CrewAI tool
pip install browsy-ai[openai]      # OpenAI function calling
pip install browsy-ai[autogen]     # AutoGen integration
pip install browsy-ai[smolagents]  # HuggingFace smolagents
pip install browsy-ai[all]         # All integrations
```

## Quick Start

```python
import browsy

# Parse HTML directly
dom = browsy.parse(html, 1920.0, 1080.0)
print(dom.page_type)
print(dom.suggested_actions)

# Session-based browsing
session = browsy.Session()
dom = session.goto("https://example.com")
session.type_text(19, "hello")
session.click(34)
```

## LangChain

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

## CrewAI

```python
from browsy.crewai import BrowsyTool
from crewai import Agent, Task, Crew

browsy_tool = BrowsyTool()
researcher = Agent(
    role="Web Researcher",
    goal="Find and summarize information from web pages",
    tools=[browsy_tool],
)
```

## OpenAI Function Calling

```python
from browsy.openai import get_tool_definitions, handle_tool_call

tools = get_tool_definitions()
result = handle_tool_call("browsy_browse", {"url": "https://example.com"})
```

## AutoGen

```python
from browsy.autogen import BrowsyBrowser
from autogen import ConversableAgent

browser = BrowsyBrowser()
assistant = ConversableAgent(name="web_assistant", llm_config={...})
browser.register(assistant)
```

## Smolagents

```python
from browsy.smolagents import BrowsyTool
from smolagents import CodeAgent, HfApiModel

tool = BrowsyTool()
agent = CodeAgent(tools=[tool], model=HfApiModel("Qwen/Qwen2.5-Coder-32B-Instruct"))
result = agent.run("Go to example.com and extract the heading.")
```

## Documentation

- [Full docs](https://ghostpeony.github.io/browsy/)
- [Python guide](https://ghostpeony.github.io/browsy/python.html)
- [Framework integrations](https://ghostpeony.github.io/browsy/framework-integrations.html)
- [browsy.dev](https://browsy.dev)

## License

MIT
