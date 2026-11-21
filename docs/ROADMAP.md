# Порядок реализации историй

Номера файлов в `docs/stories/` — каталог, не очередь. Очередь — этот файл.

Принцип: сначала предметная область в `crates/domain` (проверяется `cargo test` без браузера), тонкий UI в `crates/web` по мере надобности истории, Google-вход и Drive — в конце (без браузера не проверить).

Хранение и «сегодня» с самого начала за трейтами; в тестах — память. Реальный Google Drive появляется только в 001.

## Очередь

| # | Issue | История | Зачем сейчас |
|---|---|---|---|
| 1 | [#3](https://github.com/markshevchenko/buy-o-meter/issues/3) | [005](stories/005-accounts.md) Счета | Базовая сущность и остатки |
| 2 | [#4](https://github.com/markshevchenko/buy-o-meter/issues/4) | [003](stories/003-edit-categories.md) Настройка категорий | Дерево и признак применимости |
| 3 | [#5](https://github.com/markshevchenko/buy-o-meter/issues/5) | [002](stories/002-starter-categories.md) Стартовый набор | Сид пустого хранилища (без Google) |
| 4 | [#6](https://github.com/markshevchenko/buy-o-meter/issues/6) | [007](stories/007-expense.md) Расход | Первая операция и номер внутри дня |
| 5 | [#7](https://github.com/markshevchenko/buy-o-meter/issues/7) | [008](stories/008-income.md) Доход | Симметрия расходу |
| 6 | [#8](https://github.com/markshevchenko/buy-o-meter/issues/8) | [009](stories/009-transfer.md) Перевод | Два счёта, в т.ч. разные валюты |
| 7 | [#9](https://github.com/markshevchenko/buy-o-meter/issues/9) | [004](stories/004-delete-category.md) Удаление категории | Нужны операции для переноса |
| 8 | [#10](https://github.com/markshevchenko/buy-o-meter/issues/10) | [010](stories/010-lend-borrow.md) Дать / взять в долг | Контрагенты и сальдо |
| 9 | [#11](https://github.com/markshevchenko/buy-o-meter/issues/11) | [011](stories/011-debt-repayment.md) Возврат долга | Частичное погашение |
| 10 | [#12](https://github.com/markshevchenko/buy-o-meter/issues/12) | [012](stories/012-edit-delete-operation.md) Правка и удаление | Все типы уже есть |
| 11 | [#13](https://github.com/markshevchenko/buy-o-meter/issues/13) | [013](stories/013-quick-access.md) Остатки и долги | Сводки по уже готовой модели |
| 12 | [#14](https://github.com/markshevchenko/buy-o-meter/issues/14) | [014](stories/014-category-report.md) Отчёт по категориям | Сводки по уже готовой модели |
| 13 | [#15](https://github.com/markshevchenko/buy-o-meter/issues/15) | [006](stories/006-main-screen.md) Главный экран | Сборка ленты и быстрого ввода в UI |
| 14 | [#16](https://github.com/markshevchenko/buy-o-meter/issues/16) | [001](stories/001-google-sign-in.md) Вход через Google | OAuth, Drive, сессия — нужен браузер |

## Как отмечать прогресс

- Сделанная история — закрытый GitHub Issue (через `Closes #N` в PR).
- Следующая задача — первая строка очереди с ещё открытым Issue.
- Статус смотреть в Issues (`gh issue list`), а не править этот файл под «текущую».
