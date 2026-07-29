# LauncherPrestarter

Это престартер для GravitLauncher, написанный на языке Rust с использованием [tauri](https://v2.tauri.app/)

## Клонирование репозитория

```bash
git clone -b rust/5.7.x https://github.com/GravitLauncher/LauncherPrestarter.git
```

## Подготовка окружения (Windows)

- Установите [Visual Studio](https://visualstudio.microsoft.com/) (не Vistal Studio Code) с компонентом "Разработка приложений на C++"
- Следуйте [инструкции](https://rust-lang.org/tools/install/) и по установке окружения для разработки на Rust
- Установите [NodeJS](https://nodejs.org/en/download/current)
- Установите yarn с помощью npm
```bash
npm install --global yarn
```
- Откройте папку с престартером в консоли и выполните следующую команду:
```
yarn
```

## Отладка и сборка

Выполните `yarn tauri dev` что бы запустить престартер в режиме отладки. Престартер всегда в таком случае будет показывать окно скачки (для удобства отладки). Если вам необходимо что бы престартер не начинал скачивание Java, закомментируйте строчку `setTimeout(startDownload, appConfig.download.initialDelay);` в `src/App.svelte`. Не забудьте потом вернуть эту строчку обратно!

Выполните `yarn tauri build` для сборки итогового exe файла. Он будет лежать в `src-tauri/target/release`

## Редактирование дизайна

### Настройка IDE

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

### Архитектура проекта

В папке `src` находится исходный код фронтенда(по сути, проект на Svelte который собирается в html/css/js с помощью vite)

В папке `src-tauri` находится исходный код бекенда(явдяющийся Rust приложением)

Полезные ссылки:

- [Svelte](https://svelte.dev/)
- [Tauri](https://v2.tauri.app/)
- [Rust](https://rust-lang.org/)
- [CSS](https://developer.mozilla.org/en-US/docs/Web/CSS)
- [HTML](https://developer.mozilla.org/en-US/docs/Web/HTML)
- [JavaScript](https://developer.mozilla.org/en-US/docs/Web/JavaScript)

### Смена иконки

Логотип, отображаемый внутри приложения находится в `src/lib/assets/images/logo.svg`

Для замены лого в панели задач выполните команду

```bash
yarn tauri icon PATH_TO_ICON_PNG
```

---

## Режим скачивания лаунчера (форк Alterra)

По умолчанию (как в оригинале) `Launcher.jar` **дописывается в конец exe** модулем
`Prestarter_module`, и престартер запускает сам себя как jar. Файл получается
тяжёлым (~23 МБ), и при каждом обновлении лаунчера игрокам нужен новый exe.

Этот форк умеет **скачивать `Launcher.jar` с сервера**. Тогда exe остаётся
~5 МБ, склейка не нужна, а обновлённый лаунчер подхватывается сам — без
раздачи нового exe.

### Как включить

URL зашивается на этапе сборки (чтобы exe остался одним файлом без конфигов):

```bash
PRESTARTER_LAUNCHER_URL=https://launcher.example.com/Launcher.jar yarn tauri build
```

Переменные:

| Переменная | Обязательна | Что делает |
|---|---|---|
| `PRESTARTER_LAUNCHER_URL` | да, для режима скачивания | Адрес `Launcher.jar`. Если не задан — работает как оригинал (jar внутри exe) |
| `PRESTARTER_LAUNCHER_SHA256` | нет | Жёстко зашитый SHA-256 jar-а. Скачанный файл обязан совпасть, иначе запуск отменяется |

LaunchServer раздаёт jar по адресу вида `http(s)://host:port/Launcher.jar`.

### Как это работает

1. Java уже стоит и jar в кэше актуален → лаунчер стартует сразу, окно не показывается.
2. Иначе показывается окно с прогрессом: при необходимости качается Java, затем jar.
3. Jar кладётся в `%APPDATA%/GravitLauncherStore/Launcher.jar`, скачивание идёт во
   временный файл — оборванная загрузка не затрёт рабочий лаунчер.
4. Java запускается как `java -jar <скачанный jar>` вместо `java -jar <сам exe>`.

### Проверка целостности и обновления

Порядок такой:

1. Если задан `PRESTARTER_LAUNCHER_SHA256` — сверяется с ним (самый строгий вариант,
   но при каждом обновлении лаунчера нужно пересобирать exe).
2. Иначе запрашивается `<URL>.sha256` рядом с jar-ом — по нему определяется, что
   лаунчер обновился, и файл перекачивается.
3. Если сервер недоступен и хэша нет — используется закэшированный jar (чтобы
   лаунчер запускался и без сети).

Файл `Launcher.jar.sha256` можно генерировать на сервере после каждой сборки:

```bash
sha256sum Launcher.jar | cut -d' ' -f1 > Launcher.jar.sha256
```

> **Важно про безопасность.** Скачанный jar выполняется на машине игрока.
> Хэш, полученный по обычному `http://`, защищает только от повреждения файла:
> тот, кто может подменить jar, подменит и хэш. Поэтому раздавайте jar по
> **`https://`** — тогда TLS подтверждает подлинность сервера. Если HTTPS нет,
> используйте `PRESTARTER_LAUNCHER_SHA256` и пересобирайте exe на каждое
> обновление лаунчера.
