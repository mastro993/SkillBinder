# pstack role and model configuration
# Host: OMP v18.2.6 (project: SkillBinder)
#
# OMP dispatch selects only the agent name; the model is resolved from
# `.omp/config.yml` (`modelRoles` + `task.agentModelOverrides`). Therefore each
# entry below is a live agent name, and the parenthesised value is the concrete
# model that agent currently resolves to. Do not pass a model id as `agent`.
#
# Agent roster verified against the live `task` facility:
#   scout             -> deepseek/deepseek-v4-flash:high
#   librarian         -> deepseek/deepseek-v4-flash:high
#   designer          -> openai-codex/gpt-5.6-terra:high
#   task              -> openai-codex/gpt-5.6-luna:high
#   reviewer          -> openai-codex/gpt-5.6-sol:xhigh
#   security-reviewer -> openai-codex/gpt-5.6-sol:xhigh
#   sonic             -> deepseek/deepseek-flash:low
#
# `inherit-parent` entries mean the panel member may run on the session model
# (currently deepseek/deepseek-flash) instead of a distinct specialist.
# Canonical `planner` maps to `designer`; canonical `researcher` maps to `librarian`.
feature, refactoring: task (@pstack_implement)
bug-fix: reviewer (@pstack_review)
perf-issue: reviewer (@pstack_review)
hillclimb: task (@pstack_implement)
judgment and prose: reviewer (@pstack_review)
hardest tasks: inherit-parent
how explorer: scout (@pstack_explore)
how explainer: task (@pstack_implement)
how critics: reviewer, designer, task, inherit-parent
why investigators: librarian, scout
why synthesizer: reviewer (@pstack_review)
reflect tooling: librarian (@pstack_explore)
reflect judgment: reviewer (@pstack_review)
reflect divergent: designer (@pstack_design)
reflect synthesizer: reviewer (@pstack_review)
arena runners: designer, task, inherit-parent
arena cross-judge pool: reviewer, designer, inherit-parent
swarm workers: task (@pstack_implement)
architect runners: designer, task, reviewer, inherit-parent
interrogate reviewers: reviewer, designer, task, inherit-parent
