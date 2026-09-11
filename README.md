# DLSS5oneclick

**Русский** | [English](README_EN.md)

<p>
  <a href="https://github.com/branch-danya-dev/DLSS5oneclick/releases/latest"><img src="https://img.shields.io/github/v/release/branch-danya-dev/DLSS5oneclick?style=flat-square&color=2878D0&label=Download" alt="Скачать этот форк"></a>
  <img src="https://img.shields.io/github/downloads/branch-danya-dev/DLSS5oneclick/total?style=flat-square&color=16A34A&label=Fork%20downloads" alt="Загрузки форка">
  <a href="https://github.com/faisalkindi/DLSS5oneclick"><img src="https://img.shields.io/badge/Upstream-faisalkindi%2FDLSS5oneclick-6B7280?style=flat-square&logo=github" alt="Оригинальный репозиторий"></a>
  <a href="https://ko-fi.com/kindiboy"><img src="https://img.shields.io/badge/Support%20upstream-Ko--fi-FF5E5B?style=flat-square&logo=ko-fi&logoColor=white" alt="Поддержать автора оригинала"></a>
</p>

> [!IMPORTANT]
> **Этот репозиторий — community fork проекта [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick).**  
> Оригинальное приложение, его архитектура и основная функциональность созданы **Faisal Bahashwan (`faisalkindi`)**. Этот форк поддерживается `branch-danya-dev` и добавляет дополнительные QoL-возможности поверх upstream-проекта. Это не официальный релиз оригинального DLSS5oneclick.

## Изменения этого форка

### v0.13.16 — хоткеи для игровых коллекций

[Релиз v0.13.16](https://github.com/branch-danya-dev/DLSS5oneclick/releases/tag/v0.13.16)

- **Write hotkeys** теперь записывает выбранные сочетания клавиш во все отмеченные игры коллекции, а не только в текущий выбранный exe.
- После каждой успешной установки **Install** те же хоткеи автоматически применяются к установленной под-игре.
- Это исправляет сценарии с коллекциями вроде Mass Effect Legendary Edition, где ME1 / ME2 / ME3 находятся внутри одной записи лаунчера или одной библиотечной папки.

### v0.13.15 — поддержка коллекций из нескольких игр

Некоторые релизы Steam/Epic представляют собой сборники, в которых несколько отдельных игр находятся в одной библиотечной папке. Раньше DLSS5oneclick выбирал один exe — обычно самый большой — поэтому, например, Mass Effect Legendary Edition мог настраиваться только для ME3.

Начиная с **v0.13.15**:

- Распознаются коллекции вроде Mass Effect Legendary Edition с отдельными под-играми ME1 / ME2 / ME3.
- На странице **Setup** появляются чекбоксы найденных под-игр; по умолчанию выбраны все.
- **Install** и **Remove** выполняются для каждой отмеченной игры.
- **Update** с карточки на странице Games устанавливает DLSS5 во все найденные под-игры, а не только рядом с одним exe.

### Настраиваемые хоткеи

На странице **Setup** появилась секция **Hotkeys**, позволяющая менять основные сочетания клавиш прямо из приложения, без ручного редактирования конфигурационных файлов.

| Действие | Параметр конфигурации | По умолчанию / примечание |
|---|---|---|
| **ReShade overlay** | `[INPUT] KeyOverlay` | Поддерживает Ctrl / Shift / Alt |
| **Toggle NR** | `[RenoDX.DLSS5] NRToggleKey` | F6 |
| **NR screenshot** | `[RenoDX.DLSS5] NRScreenshotKey` | F5 |
| **OptiScaler menu** | `[Menu] ShortcutKey` | Хоткей меню OptiScaler |

Нажмите кнопку нужного хоткея, затем нужную клавишу и выберите **Write hotkeys**. **Reset** возвращает значение по умолчанию, а **Clear** удаляет привязку.

Для **32-bit игр** ReShade работает через 64-битный helper, поэтому конфигурация может находиться в `host64\ReShade.ini`. На странице Setup показывается предупреждение, если используется такой вариант.

### Автообновление отключено

Автоматическое self-update намеренно **отключено в этом форке**, чтобы официальный upstream-релиз не мог заменить модифицированный exe сборкой `faisalkindi/DLSS5oneclick`.

- При запуске больше не выполняется проверка релизов upstream.
- Действие **Check for updates** удалено; в About указано, что self-update выключен.
- `--update` только сообщает, что обновление отключено.
- Поведение контролируется флагом `update::ENABLED = false` в `src/update.rs`.

Если self-update будет возвращён в будущем, источником релизов уже настроен этот форк (`branch-danya-dev/DLSS5oneclick`), а не upstream.

**Скачать этот форк:** [последний релиз](https://github.com/branch-danya-dev/DLSS5oneclick/releases/latest) → `dlss5oneclick.exe`.

Оригинальный проект и его официальные релизы находятся в [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick).

---

DLSS5oneclick позволяет одной кнопкой установить **утёкшую сборку DLSS 5 Neural Rendering** в игры DirectX 11/12 — как с собственной поддержкой DLSS, так и без неё. Это один нативный Windows exe без внешнего runtime. Устанавливаемые компоненты загружаются из проектов, которым они принадлежат.

## Два сценария установки

| Игра | Что устанавливается |
|---|---|
| **Имеет собственный DLSS** (`nvngx_dlss.dll`, Streamline `sl.*.dll`, `nvngx_dlssg/dlssd.dll` и т. п.) | ReShade с поддержкой add-on + DLSS 5 add-on (`renodx-dlss5.addon64`, `nvngx_dlssnr.dll`). Для **DX11** также используется [dlss5-bridge](https://github.com/NIGos/dlss5-bridge). Feeder и LumeniteFX в этом сценарии не требуются. |
| **Не имеет DLSS** | Полный Feeder-сценарий: ReShade + shader headers + DLSS5-Feeder + LumeniteFX + DLSS 5 add-on + конфигурация. |

### Выбор движка для игр с нативным DLSS

По умолчанию используется ReShade + RenoDX add-on. Второй вариант — [Dagherbou's OptiScaler_DLSSNR fork](https://github.com/Dagherbou/OptiScaler_DLSSNR), который можно выбрать в GUI или параметром `--engine=opti`.

Инструмент распаковывает OptiScaler в папку игры как `dxgi.dll`, добавляет `nvngx_dlssnr.dll` и отслеживает установленные файлы, чтобы **Remove** мог удалить их корректно. В игре меню OptiScaler по умолчанию открывается клавишей Insert. Одновременно использовать оба движка нельзя, поскольку оба загружаются через `dxgi.dll`.

### RenoDX HDR mod

Если игра распознана как поддерживаемая RenoDX, появляется чекбокс **RenoDX HDR mod**. Мод может работать рядом с DLSS 5 add-on. При использовании OptiScaler ReShade устанавливается как `ReShade64.dll`, а его загрузка включается через `[Plugins] LoadReshade=true` в `OptiScaler.ini`.

### Игры на RE Engine

Для игр на RE Engine может потребоваться [REFramework](https://github.com/praydog/REFramework), загружаемый до ReShade. Для распознанных RE Engine игр DLSS5oneclick устанавливает необходимый `dinput8.dll` и при удалении трогает только тот файл, который установил сам.

### Неверно определился сценарий?

Определение DLSS основано на сканировании папок. Оставшийся от другого инструмента `nvngx_dlss.dll` может заставить игру без DLSS выглядеть как игру с нативным DLSS.

Переопределить выбор можно через:

- **Auto / Force no-DLSS (Feeder) / Force native DLSS** в GUI;
- `--mode=feeder|native`;
- переменную `DLSS5ONECLICK_MODE`.

### Ноутбуки и системы с несколькими GPU

На системах с iGPU и NVIDIA dGPU Windows может запустить игру на неправильном адаптере. DLSS5oneclick записывает то же GPU preference, которое использует Windows Settings (`GpuPreference=2;`), для exe игры и, для 32-bit игр, 64-битного helper-процесса. **Remove** откатывает только значение, созданное самим инструментом.

DX11/DX12 определяется по executable/imports и типовым структурам движков. Если API определить невозможно, предполагается DX12 и это отображается в статусе. Команда

```text
dlss5oneclick.exe "<папка игры>" --check
```

показывает определённый режим, API и план установки без изменения файлов игры.

## Сценарий для игр без DLSS

Компоненты берутся из актуальных релизов или исходников соответствующих проектов. Для игр без нативного DLSS инструмент следует схеме установки DLSS5-Feeder:

| Шаг | Что | Источник |
|---|---|---|
| 1 | ReShade **с поддержкой add-on** как `dxgi.dll` | [reshade.me](https://reshade.me) |
| 2 | Заголовки шейдеров ReShade | [crosire/reshade-shaders](https://github.com/crosire/reshade-shaders/tree/slim/Shaders) |
| 3 | `dlss5-feed.addon64` + `DLSS5_Feed.fx` | [jlrouzies-fr/DLSS5-Feeder](https://github.com/jlrouzies-fr/DLSS5-Feeder/releases/latest) |
| 4 | Motion-vector provider | [umar-afzaal/LumeniteFX](https://github.com/umar-afzaal/LumeniteFX) |
| 5 | DLSS 5 add-on и NVIDIA runtimes | [RankFTW/rhi-repo](https://github.com/RankFTW/rhi-repo/releases) |
| 6 | Настройка `ReShade.ini` / preset | выполняется DLSS5oneclick |

При повторном запуске скачиваются только отсутствующие компоненты.

## Использование

1. Запустите `dlss5oneclick.exe`.
2. На странице **Games** отображаются найденные игры Steam, Epic Games, GOG и Xbox / Game Pass. Через **Add a folder** / **Add a game** можно добавить папку или exe вручную. `--list-games` выводит тот же список в CLI.
3. Откройте страницу **Setup** нужной игры. Инструмент найдёт игровой exe, включая распространённые вложенные структуры каталогов.
4. Если запись является **коллекцией** вроде Mass Effect Legendary Edition, каждая найденная под-игра отображается отдельным чекбоксом. По умолчанию выбраны все; снимите галочку с тех игр, которые не нужно изменять.
5. При необходимости настройте дополнительные компоненты и **Hotkeys**.
6. Нажмите **Install DLSS 5**. Для коллекций установка выполняется для каждой отмеченной под-игры, а выбранные хоткеи применяются после каждой успешной установки.
7. В игре откройте ReShade (**Home**) → **Add-ons** → **DLSS 5 Neural Rendering** и включите Neural Rendering. MSAA/SSAA игры рекомендуется отключить.

По умолчанию **F6** включает/выключает Neural Rendering, а **F5** сохраняет screenshot add-on. Оба хоткея можно переназначить на странице Setup → **Hotkeys**.

CLI:

```text
dlss5oneclick.exe "C:\Games\Foo"
dlss5oneclick.exe "C:\Games\Foo" --renodx
dlss5oneclick.exe "C:\Games\Foo" --check
dlss5oneclick.exe "C:\Games\Foo" --diagnose
dlss5oneclick.exe "C:\Games\Foo" --remove
```

## Обновления

Self-update **отключён в этом форке**. Приложение не проверяет и не устанавливает новые версии автоматически. `dlss5oneclick.exe --update` сообщает, что автоматическое обновление отключено, и завершает работу.

Релизы форка публикуются вручную в [branch-danya-dev/DLSS5oneclick Releases](https://github.com/branch-danya-dev/DLSS5oneclick/releases). Официальные релизы оригинального проекта находятся в [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick/releases).

## Загрузка компонентов и GitHub

Компоненты скачиваются из upstream-проектов. Если GitHub недоступен из вашей сети или региона, для загрузки может потребоваться VPN или прокси.

## Поддержка GPU

Инструмент проверяет установленные графические адаптеры и заранее блокирует заведомо неподдерживаемые конфигурации — например, не-NVIDIA GPU и NVIDIA без tensor cores. Разные поколения RTX имеют разную стоимость Neural Rendering; интерфейс показывает определённый уровень поддержки.

Виртуальные и удалённые адаптеры считаются неизвестными и допускаются. Если GPU определён неправильно, проверку можно обойти переменной:

```text
DLSS5ONECLICK_SKIP_GPU_CHECK=1
```

## Проверка скачанного файла

В описании каждого релиза этого форка публикуется SHA-256 файла `dlss5oneclick.exe`.

Проверка через Windows:

```text
certutil -hashfile dlss5oneclick.exe SHA256
```

или через PowerShell:

```text
Get-FileHash .\dlss5oneclick.exe
```

Бинарные сборки этого форка следует скачивать только из [Releases этого репозитория](https://github.com/branch-danya-dev/DLSS5oneclick/releases). Официальные upstream-сборки находятся в [Releases оригинального проекта](https://github.com/faisalkindi/DLSS5oneclick/releases).

## Windows Defender / SmartScreen

Exe не подписан сертификатом издателя и устанавливает DLL-файлы в папки игр, поэтому Windows может показывать предупреждение Unknown publisher или помещать отдельные файлы в карантин. Все релизы этого форка собираются из публичного исходного кода этого репозитория.

## Известные ограничения

- **Feeder + exclusive fullscreen.** При смене фокуса игра может пересоздать swapchain, из-за чего DLSS5-Feeder пересобирает DLSS feature. Для затронутых игр borderless/windowed обычно стабильнее.
- **Стоимость кадров.** Neural Rendering в высоком нативном разрешении может добавлять несколько миллисекунд времени кадра. При включённом v-sync это способно привести к резким ступеням FPS.
- **API иногда определяется как unknown.** Некоторые exe загружают D3D динамически. В таком случае инструмент предполагает DX12; результат можно проверить через `--check`.
- DLSS 5 add-on/model, распространяемые через указанный community-источник, закрытые; инструмент не может гарантировать безопасность сторонних бинарных файлов.

## Что не поддерживается / особенности

- **32-bit игры** поддерживаются по Feeder-сценарию через 32-битные ReShade/add-on и helper в `host64\`. Этот путь рассчитан на D3D11.
- **DirectX 9** и **Vulkan** текущим workflow не поддерживаются.
- Онлайн-игры с распознанными anti-cheat системами по умолчанию блокируются. Использовать override следует только если anti-cheat действительно отключён, и такую модифицированную установку не следует запускать в онлайн-режиме.

## Разработка

Rust 2021, один crate. GUI — egui/eframe; HTTP — reqwest (rustls); работа с архивами — crate `zip`.

```text
cargo test
cargo build --release   # target/release/dlss5oneclick.exe
```

Тесты используют локальные fake-данные и не требуют сети.

## Благодарности

Этот форк основан на оригинальном DLSS5oneclick и автоматизирует работу нескольких сторонних проектов. Основные авторы и проекты:

- **[Faisal Bahashwan (`faisalkindi`)](https://github.com/faisalkindi)** — автор оригинального [DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick), на котором основан этот форк.
- **[crosire](https://github.com/crosire)** — [ReShade](https://reshade.me) и [reshade-shaders](https://github.com/crosire/reshade-shaders).
- **[jlrouzies-fr](https://github.com/jlrouzies-fr)** — [DLSS5-Feeder](https://github.com/jlrouzies-fr/DLSS5-Feeder).
- **[Afzaal (Kaidō)](https://github.com/umar-afzaal)** — [LumeniteFX](https://github.com/umar-afzaal/LumeniteFX).
- **[praydog](https://github.com/praydog)** — [REFramework](https://github.com/praydog/REFramework).
- **[clshortfuse](https://github.com/clshortfuse)** и RenoDX community — [RenoDX](https://github.com/clshortfuse/renodx).
- **[RankFTW](https://github.com/RankFTW)** — [RHI](https://github.com/RankFTW/RHI) и [rhi-repo](https://github.com/RankFTW/rhi-repo).
- **NVIDIA** — DLSS 5 и NVIDIA runtimes.
- **[Dagherbou](https://github.com/Dagherbou)** и **[OptiScaler team](https://github.com/optiscaler/OptiScaler)** — интеграция OptiScaler / DLSSNR.
- **[NIGos](https://github.com/NIGos)** — [dlss5-bridge](https://github.com/NIGos/dlss5-bridge).
- **[emilk](https://github.com/emilk)** — [egui / eframe](https://github.com/emilk/egui).
- Шрифты: [Sora](https://github.com/sora-xor/sora-font), [IBM Plex Sans](https://github.com/IBM/plex), [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) — SIL OFL.

## Лицензия

Этот форк сохраняет upstream-лицензию **MIT License** и оригинальный copyright notice. Каждый скачиваемый сторонний компонент сохраняет собственную лицензию и условия использования.