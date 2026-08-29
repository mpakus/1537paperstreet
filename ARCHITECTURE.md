# Архитектура

1537paperstreet — четыре слоя. Полные контракты и бюджеты — в
[`docs/PLAN.md`](docs/PLAN.md). Решения фиксируются в [`docs/adr/`](docs/adr/).

## Слои

- **`ps-core`** — конфигурация (включая `agents`), реестр проектов, атомарное
  JSON-хранилище, безопасные файловые операции (`fsops::resolve` — единственная
  дверь в проект), дерево, FSEvents-наблюдатель, чтение документов (`docio`),
  каталог тем, кэш SVG Mermaid, состояние UI, история промптов Assistant,
  дашборд, ротация логов. Не зависит от Tauri.
- **`ps-render`** — потоковый конвейер Markdown → HTML на `pulldown-cmark`.
  Уникальные идентификаторы заголовков, TOC, шаблоны Mermaid с BLAKE3-хешем,
  span'ы KaTeX, `asset://` только после `fsops::resolve`, санитайзинг
  `ammonia`. Не зависит от Tauri.
- **`ps-app`** — оболочка Tauri 2: тонкие IPC-команды, overlay-титлбар,
  нативное меню, протоколы `asset://` / `doc://` / `fs://`, `WatchHub`,
  ACP-хост (в т.ч. Codex `app-server --stdio`). `save_user_file` пишет только
  на абсолютный путь из нативного Save (`.png` / `.svg` / `.pdf`), это не
  путь проекта.
- **`ui`** — Svelte 5. Панели проектов, дерева, превью, Assistant, Dashboard;
  настройки; быстрое открытие. Бизнес-логика в Rust. Исключения: ленивый
  рендер уже санитизированных шаблонов Mermaid/KaTeX и буфер CodeMirror
  (P9).

## Данные на диске

| Путь                                     | Назначение                                    |
| ---------------------------------------- | --------------------------------------------- |
| `~/.1537paperstreet/config.json`         | Настройки, включая `agents`                   |
| `~/.1537paperstreet/projects.json`       | Реестр проектов                               |
| `~/.1537paperstreet/ui-state.json`       | Раскрытые узлы, ширины панелей                |
| `~/.1537paperstreet/agents/prompts.json` | История промптов Assistant (не ответы агента) |
| `~/.1537paperstreet/themes/`             | Пользовательские темы                         |
| `~/.1537paperstreet/cache/mermaid/`      | Кэш SVG диаграмм                              |
| `~/.1537paperstreet/logs/app.log`        | Логи без текста документов и промптов         |

Запись служебных JSON и пользовательских файлов — атомарная
(temp → fsync → rename).
