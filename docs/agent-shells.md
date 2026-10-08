# Agent Shells

Every shell agent's terminal runs your own login shell, interactively (`$SHELL -i`), in the agent's folder. It reads your rc files, so the agent gets the same `PATH`, aliases and tools as a terminal you opened yourself.

That also means it runs everything else your rc files start. Most of it is harmless, but a prompt theme that starts a background daemon per shell starts one per agent.

## `KNOT_AGENT`

Knot sets `KNOT_AGENT=1` in every agent terminal's environment. Test it in your dotfiles to skip anything only a person at a prompt needs:

```zsh
if [[ -n $KNOT_AGENT ]]; then
  # in an agent's terminal
fi
```

## powerlevel10k

powerlevel10k starts a `gitstatusd` daemon, plus two helper `zsh` processes, for every interactive shell, to draw the prompt's git segment. Agents don't read the prompt, and Knot tracks git itself, so in an agent terminal those processes do nothing useful.

Add this line to `~/.p10k.zsh`, **after** the `unset -m '(POWERLEVEL9K_*|DEFAULT_USER)~POWERLEVEL9K_GITSTATUS_DIR'` line near the top of the file:

```zsh
  [[ -n $KNOT_AGENT ]] && typeset -g POWERLEVEL9K_DISABLE_GITSTATUS=true
```

It has to come after that line. The config `p10k configure` generates starts by unsetting every `POWERLEVEL9K_*` variable, including any Knot could set from outside, which is why Knot only marks the shell and leaves the choice to you. Your own terminals are unaffected: `KNOT_AGENT` is unset there, and their prompts keep the git segment.
