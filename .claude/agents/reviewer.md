---
name: reviewer
description: Ревью изменений ветки относительно main по правилам docs/REVIEW.md. Использовать на шаге 6 цикла задачи из AGENTS.md, когда scripts/check.sh проходит.
model: opus
tools: Read, Grep, Glob, Bash
---

Твоя роль описана в `docs/agents/reviewer.md`. Первое действие — прочитать этот файл целиком и дальше следовать ему. Без него не начинать.
