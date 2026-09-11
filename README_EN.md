# DLSS5oneclick

[**Русский**](README.md) | **English**

<p>
  <a href="https://github.com/branch-danya-dev/DLSS5oneclick/releases/latest"><img src="https://img.shields.io/github/v/release/branch-danya-dev/DLSS5oneclick?style=flat-square&color=2878D0&label=Download" alt="Download this fork"></a>
  <img src="https://img.shields.io/github/downloads/branch-danya-dev/DLSS5oneclick/total?style=flat-square&color=16A34A&label=Fork%20downloads" alt="Fork downloads">
  <a href="https://github.com/faisalkindi/DLSS5oneclick"><img src="https://img.shields.io/badge/Upstream-faisalkindi%2FDLSS5oneclick-6B7280?style=flat-square&logo=github" alt="Upstream repository"></a>
  <a href="https://ko-fi.com/kindiboy"><img src="https://img.shields.io/badge/Support%20upstream-Ko--fi-FF5E5B?style=flat-square&logo=ko-fi&logoColor=white" alt="Support upstream author"></a>
</p>

> [!IMPORTANT]
> **This repository is a community fork of [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick).**  
> The original application, architecture and core functionality were created by **Faisal Bahashwan (`faisalkindi`)**. This fork is maintained by `branch-danya-dev` and adds quality-of-life changes on top of the upstream project. It is not an official upstream release.

## Fork-specific changes

### v0.13.16 — hotkeys for collections

[Release v0.13.16](https://github.com/branch-danya-dev/DLSS5oneclick/releases/tag/v0.13.16)

- **Write hotkeys** now writes the selected bindings into every selected game in a collection, not only the currently inspected target.
- After each successful **Install**, the same hotkeys are also applied to that installed sub-game.
- This fixes collection workflows such as Mass Effect Legendary Edition where ME1 / ME2 / ME3 live under one launcher/library entry.

### v0.13.15 — multi-game collections

Some Steam/Epic releases are collections that contain several real games in one library folder. Previously DLSS5oneclick selected one executable — usually the largest one — so a collection such as Mass Effect Legendary Edition could end up configured only for ME3.

Starting with **v0.13.15**:

- Collections such as Mass Effect Legendary Edition are detected as multiple sub-games (ME1 / ME2 / ME3).
- The **Setup** page shows checkboxes for the detected sub-games; all are selected by default.
- **Install** and **Remove** run for every selected sub-game.
- **Update** from a Games card installs into all detected sub-games instead of only one executable.

### Configurable hotkeys

The **Setup** page includes a **Hotkeys** section for editing the shortcuts used by the installed components without manually opening their configuration files.

| Action | Config entry | Default / notes |
|---|---|---|
| **ReShade overlay** | `[INPUT] KeyOverlay` | Supports Ctrl / Shift / Alt modifiers |
| **Toggle NR** | `[RenoDX.DLSS5] NRToggleKey` | F6 |
| **NR screenshot** | `[RenoDX.DLSS5] NRScreenshotKey` | F5 |
| **OptiScaler menu** | `[Menu] ShortcutKey` | OptiScaler shortcut |

Click a hotkey button, press the key you want, then choose **Write hotkeys**. **Reset** restores the default value and **Clear** removes the binding.

For **32-bit games**, ReShade runs through the 64-bit helper and its configuration may live at `host64\ReShade.ini`; the Setup page displays a warning when this applies.

### Self-update disabled

Automatic self-update is deliberately **disabled in this fork** so an upstream release cannot replace the modified executable with an official `faisalkindi/DLSS5oneclick` build.

- Startup no longer checks upstream releases.
- The **Check for updates** action is removed; About notes that self-update is disabled.
- `--update` only reports that updating is disabled.
- The implementation is controlled by `update::ENABLED = false` in `src/update.rs`.

If self-update is enabled again in the future, the configured release source is this fork (`branch-danya-dev/DLSS5oneclick`), not upstream.

**Download this fork:** [latest release](https://github.com/branch-danya-dev/DLSS5oneclick/releases/latest) → `dlss5oneclick.exe`.

For the original project and its official releases, use [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick).

---

One button that sets up the **leaked DLSS 5 neural-rendering build** in any DirectX 11/12 game, with or without DLSS of its own. Single native Windows exe, no runtime. Everything it installs is downloaded from the projects that made it; the only third-party content inside the exe is three SIL-OFL fonts.

## Two paths, picked automatically

| The game | What gets installed |
|---|---|
| **Ships its own DLSS** (an `nvngx_dlss.dll` this tool did not place, or Streamline `sl.*.dll`, `nvngx_dlssg/dlssd.dll`, anywhere up to four folders deep, or under an Unreal project's `Plugins` tree) | ReShade add-on build + the DLSS 5 add-on (`renodx-dlss5.addon64`, `nvngx_dlssnr.dll`). The add-on hooks the game's own NGX calls directly. **DX11 games** also get [dlss5-bridge](https://github.com/NIGos/dlss5-bridge), which replays the D3D11 DLSS calls on a private D3D12 device so the add-on can see them. No Feeder, no LumeniteFX; a Feeder left over from an earlier run is removed. |
| **Has no DLSS** | The full Feeder path below: ReShade + shader headers + DLSS5-Feeder + LumeniteFX + the DLSS 5 add-on + config. |

**Engine choice for games with native DLSS.** The default engine is ReShade + the RenoDX add-on. A second engine — [Dagherbou's OptiScaler_DLSSNR fork](https://github.com/Dagherbou/OptiScaler_DLSSNR) — can be selected in the GUI or with `--engine=opti`. The tool extracts the fork into the game as `dxgi.dll`, adds `nvngx_dlssnr.dll`, and tracks the installed files so Remove can cleanly remove them. In game, Insert opens the OptiScaler overlay; Neural Rendering is off by default there. The two engines cannot share a game because both load as `dxgi.dll`.

**RenoDX HDR mod (optional).** When the tool recognises a game supported by RenoDX, a **RenoDX HDR mod** checkbox appears. The mod can run beside the DLSS 5 add-on. On the OptiScaler engine the tool also installs ReShade as `ReShade64.dll` and enables it through `[Plugins] LoadReshade=true` in `OptiScaler.ini`.

**RE Engine games.** Games using RE Engine may require [REFramework](https://github.com/praydog/REFramework) to load before ReShade. For recognised RE Engine games DLSS5oneclick installs the required `dinput8.dll` first and removes only the file it placed itself.

**Wrong path?** The DLSS detection is a folder scan, so a stray `nvngx_dlss.dll` can make a game without DLSS look like a native-DLSS game. Use **Auto / Force no-DLSS (Feeder) / Force native DLSS**, `--mode=feeder|native`, or `DLSS5ONECLICK_MODE` to override detection.

**Hybrid machines.** On systems with an iGPU and an NVIDIA dGPU, Windows may start the game on the wrong adapter. DLSS5oneclick writes the same GPU preference used by Windows Settings (`GpuPreference=2;`) for the selected game executable and, for 32-bit games, the 64-bit helper. Remove reverts only the value written by the tool.

DX11 vs DX12 is detected from the executable/imports and common engine layouts. When detection cannot determine the API, DX12 is assumed and the status line says so. `dlss5oneclick.exe "<game folder>" --check` prints the detected mode, API and install plan without installing anything.

### The no-DLSS path

Every component is taken from its own project's current release or source. For games without native DLSS, the tool follows the DLSS5-Feeder installation model:

| Step | What | From |
|---|---|---|
| 1 | ReShade **with add-on support**, dropped as `dxgi.dll` | [reshade.me](https://reshade.me) |
| 2 | ReShade shader headers | [crosire/reshade-shaders](https://github.com/crosire/reshade-shaders/tree/slim/Shaders) |
| 3 | `dlss5-feed.addon64` + `DLSS5_Feed.fx` | [jlrouzies-fr/DLSS5-Feeder](https://github.com/jlrouzies-fr/DLSS5-Feeder/releases/latest) |
| 4 | Motion-vector provider files | [umar-afzaal/LumeniteFX](https://github.com/umar-afzaal/LumeniteFX) |
| 5 | DLSS 5 add-on and NVIDIA runtimes | [RankFTW/rhi-repo](https://github.com/RankFTW/rhi-repo/releases) |
| 6 | `ReShade.ini` / preset configuration | written by this tool |

A re-run downloads only what is missing.

## Use

1. Run `dlss5oneclick.exe`.
2. The **Games** page lists detected Steam, Epic Games, GOG and Xbox / Game Pass installs. **Add a folder** / **Add a game** can also add a folder or executable manually. `--list-games` prints the same list headless.
3. Open a game's **Setup** page. DLSS5oneclick searches the folder and common nested layouts for the real game executable.
4. If the library entry is a **collection**, such as Mass Effect Legendary Edition, each detected sub-game is shown with a checkbox. All are selected by default; uncheck any title you do not want to modify.
5. Configure optional components and **Hotkeys** as needed.
6. Click **Install DLSS 5**. For collections, installation runs for every selected sub-game and applies the selected hotkeys to each successful install.
7. In game: open ReShade (**Home**) → **Add-ons** → **DLSS 5 Neural Rendering** and enable it. Keep the game's MSAA/SSAA off.

**F6** toggles neural rendering on/off and **F5** saves the add-on screenshot by default. Both are remappable on the Setup page under **Hotkeys**.

CLI: `dlss5oneclick.exe "C:\Games\Foo"` / `--renodx` / `--check` / `--diagnose` / `--remove`.

## Updates

Self-update is **disabled in this fork**. The application does not check or install releases automatically. `dlss5oneclick.exe --update` reports that updating is disabled and exits.

Manual releases of this fork are published at [branch-danya-dev/DLSS5oneclick Releases](https://github.com/branch-danya-dev/DLSS5oneclick/releases). The original project's official releases remain available from [faisalkindi/DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick/releases).

## Downloads and GitHub

Components are downloaded from their upstream projects. Where GitHub itself is unreachable, a proxy or VPN may be required.

## GPU support

The tool checks installed display adapters and refuses known unsupported configurations such as non-NVIDIA cards and NVIDIA GPUs without tensor cores. RTX generations have different neural-rendering costs; the UI reports the detected support tier. Virtual/remote adapters are treated as unknown and allowed. Set `DLSS5ONECLICK_SKIP_GPU_CHECK=1` to bypass the refusal if detection is wrong.

## Verifying a download

Each fork release contains the SHA-256 of the attached `dlss5oneclick.exe` in its release notes. Check it with:

```text
certutil -hashfile dlss5oneclick.exe SHA256
```

or PowerShell `Get-FileHash`.

For this fork, download binaries only from this repository's [Releases](https://github.com/branch-danya-dev/DLSS5oneclick/releases). For official upstream binaries, use the original project's [Releases](https://github.com/faisalkindi/DLSS5oneclick/releases).

## Windows Defender / SmartScreen

The executable is not code-signed and downloads DLLs into game folders, so Windows heuristics may show an unknown-publisher warning or quarantine files. Every fork release is built from the public source in this repository.

## Known issues

- **Feeder path + exclusive fullscreen.** Focus changes may recreate the swapchain and force DLSS5-Feeder to rebuild its feature. Borderless/windowed mode is safer for affected games.
- **Frame cost.** Neural rendering at high native resolutions can add several milliseconds. With v-sync this may cause large frame-rate steps.
- **API detection can be unknown** for executables that load D3D dynamically. The tool assumes DX12 in that case; `--check` shows the detected result.
- The DLSS 5 add-on/model distributed by the referenced community release source is closed-source; this tool cannot vouch for third-party binaries.

## Not handled / limitations

- **32-bit games** are supported on the Feeder path through a 32-bit ReShade/add-on plus a `host64\` helper. The 32-bit path is D3D11-only.
- **DirectX 9** and **Vulkan** are refused by the current workflow.
- Online games with recognised anti-cheat systems are refused by default. Use any override only when the game's anti-cheat is genuinely disabled and do not take such a modified install online.

## Development

Rust 2021, single crate. GUI is egui/eframe; HTTP is reqwest (rustls); archives use the `zip` crate.

```text
cargo test
cargo build --release   # target/release/dlss5oneclick.exe
```

Tests use local fakes only; no network.

## Credits

This fork is based on the original DLSS5oneclick project and continues to automate work from several projects. Credit belongs to:

- **[Faisal Bahashwan (`faisalkindi`)](https://github.com/faisalkindi)** — original author of [DLSS5oneclick](https://github.com/faisalkindi/DLSS5oneclick), the upstream project this repository is forked from.
- **[crosire](https://github.com/crosire)** — [ReShade](https://reshade.me) and [reshade-shaders](https://github.com/crosire/reshade-shaders).
- **[jlrouzies-fr](https://github.com/jlrouzies-fr)** — [DLSS5-Feeder](https://github.com/jlrouzies-fr/DLSS5-Feeder).
- **[Afzaal (Kaidō)](https://github.com/umar-afzaal)** — [LumeniteFX](https://github.com/umar-afzaal/LumeniteFX).
- **[praydog](https://github.com/praydog)** — [REFramework](https://github.com/praydog/REFramework).
- **[clshortfuse](https://github.com/clshortfuse)** and the RenoDX community — [RenoDX](https://github.com/clshortfuse/renodx).
- **[RankFTW](https://github.com/RankFTW)** — [RHI](https://github.com/RankFTW/RHI) and [rhi-repo](https://github.com/RankFTW/rhi-repo).
- **NVIDIA** — DLSS 5 and the NVIDIA runtimes.
- **[Dagherbou](https://github.com/Dagherbou)** and the **[OptiScaler team](https://github.com/optiscaler/OptiScaler)** — OptiScaler / DLSSNR integration.
- **[NIGos](https://github.com/NIGos)** — [dlss5-bridge](https://github.com/NIGos/dlss5-bridge).
- **[emilk](https://github.com/emilk)** — [egui / eframe](https://github.com/emilk/egui).
- Fonts: [Sora](https://github.com/sora-xor/sora-font), [IBM Plex Sans](https://github.com/IBM/plex), [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) — SIL OFL.

## License

This fork retains the upstream **MIT License** and the original copyright notice. Each downloaded component keeps its own license and terms.