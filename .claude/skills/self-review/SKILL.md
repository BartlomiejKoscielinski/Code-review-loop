---
name: self-review
description: Przegląd własnej zmiany przed commitem. Diff względem gałęzi głównej, pliki spoza planu, testy i analizatory, opis zmiany w 3-5 zdaniach, a potem review w świeżym kontekście przez subagenta code-reviewer.
disable-model-invocation: true
allowed-tools: Read Grep Glob Bash(git status *) Bash(git diff *) Bash(git log *) Bash(git merge-base *) Bash(git rev-parse *) Bash(git symbolic-ref *) PowerShell(git status *) PowerShell(git diff *) PowerShell(git log *) PowerShell(git merge-base *) PowerShell(git rev-parse *) PowerShell(git symbolic-ref *)
---

# Self-review przed commitem

Nie commituj i nie poprawiaj kodu w trakcie tego skilla. Na końcu zapytaj, które uwagi wdrożyć.

1. **Baza porównania.** Gałąź główna to wynik `git symbolic-ref --short refs/remotes/origin/HEAD`, a jeśli go nie ma, pierwsza istniejąca z `main` i `master`. Bazą jest `git merge-base HEAD <gałąź główna>`. Jeśli repo nie ma jeszcze commitów albo gałęzi głównej, porównuj z pustym drzewem i napisz to wprost.
2. **Diff.**
   - Weź `git diff <baza>` (zmiany w commitach i niezacommitowane) oraz `git status --porcelain`. Nowe nieśledzone pliki przeczytaj w całości.
   - Podaj `git diff --stat <baza>`.
   - Powyżej ok. 400 zmienionych linii zaproponuj podział (zasada z `CLAUDE.md`).
3. **Pliki spoza planu.** Znajdź w `docs/plan/` zadanie, które realizuje zmiana. Jeśli nie da się go wskazać, zapytaj. Wypisz pliki i fragmenty, które nie wynikają z zadania.
4. **Testy i analizatory.**
   - Uruchom komendy z sekcji „Komendy” w `CLAUDE.md` dla zmienionych ekosystemów i pokaż wynik.
   - Jeśli zmiana dotyka `.claude/hooks/`, uruchom `node --test .claude/hooks/hooks.test.mjs`.
   - Jeśli dla ekosystemu nie ma jeszcze komend, napisz to wprost zamiast zgadywać.
5. **Opis zmiany** w 3–5 zdaniach: co się zmienia, dlaczego (odnośnik do zadania lub ADR), jak to zweryfikowano, jakie zostają ryzyka.
6. **Review.**
   - Wywołaj subagenta `code-reviewer`. Przekaż mu dosłowny diff (przy dużym podziel go według plików, nie streszczaj), opis zmiany, odnośnik do zadania i listę plików spoza planu.
   - Jeśli zmiana dotyka tokenu GitLaba, wywołań `claude -p`, renderowania treści MR lub sidecara, wywołaj także `security-reviewer`.
7. **Raport.** Pokaż kolejno:
   - opis zmiany;
   - pliki spoza planu;
   - wyniki testów;
   - komentarze recenzentów, od `blocking`.
   Potem zapytaj, co poprawić.
