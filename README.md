# AutoWinDrivers

Утилита для Windows 10/11 на Rust: сканирует всё оборудование, находит и помогает установить
драйверы, в фоне отслеживает подключение новых устройств и подбирает драйверы под них.

План разработки, архитектура, этапы и сроки — в [docs/PLAN.md](docs/PLAN.md),
принятые архитектурные решения — в [docs/adr](docs/adr).

## Состояние

Этап 0 (фундамент): Cargo workspace, CI, ядро модели и консольный сканер устройств.
Интерфейс (Tauri 2 + Svelte), поиск и установка драйверов, агент мониторинга — следующие этапы по плану.

## Сборка и запуск

Нужен stable Rust (см. `rust-toolchain.toml`). На Windows 10/11:

```powershell
cargo run -p awd-cli -- scan                     # таблица устройств и их драйверов
cargo run -p awd-cli -- scan --problems          # только проблемные и отключённые
cargo run -p awd-cli -- scan --all --json -o scan.json   # полный отчёт, включая отключённые от ПК устройства
cargo run -p awd-cli -- problem 28 --lang ru     # что значит код проблемы 28
```

На Linux/macOS живое сканирование недоступно, но сохранённый отчёт можно просматривать:

```sh
cargo run -p awd-cli -- scan --input tests/fixtures/scan-sample.json
```

Проверки перед коммитом: `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`,
`cargo deny check`.

## Структура

- `crates/awd-core` — платформонезависимое ядро: модель устройства, Hardware ID, коды проблем, версии драйверов.
- `crates/awd-win` — Windows-бэкенд (CfgMgr32); весь `unsafe` только здесь.
- `crates/awd-cli` — консольная утилита `awd`.
- `docs/` — план и ADR. `tests/fixtures/` — примеры отчётов.

## Лицензия

MIT OR Apache-2.0, на выбор пользователя. См. [LICENSE-MIT](LICENSE-MIT) и [LICENSE-APACHE](LICENSE-APACHE).
