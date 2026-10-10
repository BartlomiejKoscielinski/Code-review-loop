# docs/plan: wyniki planowania

Katalog wypełni sesja planowania, która czyta `docs/input/plan.md` i `docs/input/mocks/`. Ta sesja ustali:

- strukturę repo i podział na moduły (Tauri/Rust, UI React + TypeScript, sidecar .NET);
- kolejność prac jako zadania o rozmiarze do ok. 400 LOC na jedno review, każde z kryteriami akceptacji;
- komendy build, test i lint dla trzech ekosystemów. Trafią one do sekcji „Komendy” w `CLAUDE.md` i do listy `allow` w `.claude/settings.json`;
- listę spike'ów do wykonania przed implementacją (`spikes/`).

Decyzje podjęte podczas planowania zapisuje się jako ADR w `docs/adr/` (skill `/adr`). Kompletność planu sprawdza subagent `plan-critic`.
