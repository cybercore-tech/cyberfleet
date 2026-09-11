# A note on how this gets built

I use AI assistance — Claude, specifically — to help build and maintain this
project, and you'll see it in the commit history. I'm not going to pretend
otherwise, and I'm not going to bury it either.

Here's what that actually means:

## What's mine

The idea for cyberfleet came from my own workflow, not a brainstorm: I run
more repos at once than I can hold in my head, and I was tired of `cd`-ing
into each one just to remember which ones I'd forgotten about. Deciding what
this tool should actually do — status at a glance from the bar, a real TUI
for the workflow itself, fetch that behaves exactly like running `git fetch`
yourself instead of hiding what it's doing — that's mine. I spent 30+ years
as an auto tech before I came to this, and the one thing that trade teaches
you cold is that the tool in your hand doesn't make the diagnosis. You do.

I also do the thing most people skip: I run this stuff for real, on my own
repos, before I call it finished. The SSH-passphrase-prompt hang this
project found during development — `git fetch` needing your key's passphrase
opens `/dev/tty` directly, which collided with the TUI's raw-mode screen and
froze the whole app with your own keystrokes going nowhere — didn't turn up
by reading the code. It turned up because I fetched a real repo of mine and
watched it lock up. Catching that, and deciding the fix was "hand the
terminal back for that one moment" rather than papering over it, is exactly
the kind of call that's on me every time.

## What the AI does

Grunt work, mostly. Boilerplate, the actual `git2` plumbing for ahead/behind
and dirty-file counts, hardening a launcher script against patterns a
security scanner will flag, writing docs that match what the code does
instead of what I meant to build last month. It's a second set of eyes that
doesn't get tired at 1am and doesn't take it personally when I tell it the
first draft is wrong.

I don't ship what I don't understand. If I can't explain why a fix works,
I'm not done reviewing it yet.

## Why say any of this

Because I'd rather you know going in than find out later and wonder what
else wasn't said. I spent most of my working life doing things the hard way
before tools like this existed. I don't have anything to prove by pretending
I still do it that way — working smart isn't a shortcut, it's the point.

Judge the code. That's always been the right way to do it anyway.

— darkstardevx
