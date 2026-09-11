//! RU/EN UI strings. Technical identifiers and file paths stay untranslated.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Ru,
    En,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Language::Ru => "ru",
            Language::En => "en",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::Ru => "Русский",
            Language::En => "English",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T {
    // nav / chrome
    Games,
    Setup,
    Settings,
    About,
    Ready,
    LeakedBuild,
    Support,
    TipTitle,
    TipBody,
    Dismiss,

    // games page
    YourGames,
    YourGamesSub,
    TotalGames,
    InstalledCount,
    AvailableCount,
    SearchGames,
    AddGame,
    AddFolder,
    Rescan,
    OpenSetup,
    InstalledByTool,
    AddedByYou,
    NoGamesFound,
    NoGamesHint,
    StatusInstalled,
    StatusNotInstalled,
    StatusReady,
    StatusIncomplete,
    StatusUpdate,

    // setup
    GameExe,
    Inspecting,
    CollectionTitle,
    CollectionSelected,
    SelectAll,
    SelectNone,
    InstallMethod,
    EngineReshade,
    EngineReshadeDetail,
    EngineOpti,
    EngineOptiDetail,
    Recommended,
    NeuralRendering,
    StableRenodx,
    ExperimentalUpstream,
    Components,
    Install,
    InstallN,
    Remove,
    Diagnose,
    ConfirmRemove,
    ConfirmRemoveAll,
    Cancel,
    FeederKnobs,
    Hotkeys,
    HotkeysHint,
    WriteHotkeys,
    ReloadFromDisk,
    Reset,
    Clear,
    PressKey,
    OverlayReshade,
    ToggleNr,
    NrScreenshot,
    OverlayOpti,
    InstallLog,
    WriteCfg,
    Basics,
    MotionVectors,
    Advanced,

    // settings
    General,
    QualityPreset,
    FeederDefaults,
    OverlayDiagnostics,
    Language,
    ApplyToGame,
    ResetFeederDefaults,
    SaveSettings,

    // about
    AboutTitle,
    CommunityForkOf,
    ForkChanges,
    ForkHotkeys,
    ForkCollections,
    ForkHotkeyProp,
    ForkNoUpdate,
    ForkI18n,
    Credits,
    BuildInfo,
    Version,
    Architecture,
    UpdateStatus,
    SelfUpdateDisabled,
    X64,

    // common
    Path,
    Yes,
    No,
    Ok,
    Error,
    Warning,
}

impl Language {
    pub fn t(self, key: T) -> &'static str {
        match self {
            Language::Ru => ru(key),
            Language::En => en(key),
        }
    }
}

fn en(key: T) -> &'static str {
    match key {
        T::Games => "Games",
        T::Setup => "Setup",
        T::Settings => "Settings",
        T::About => "About",
        T::Ready => "Ready",
        T::LeakedBuild => "Leaked Build",
        T::Support => "Support",
        T::TipTitle => "Tip",
        T::TipBody => "Pick a game on Games, then Install on Setup. Use Settings to seed Feeder defaults for the next Install.",
        T::Dismiss => "Dismiss",
        T::YourGames => "Your games",
        T::YourGamesSub => "Install and manage DLSS 5 for games found on this PC.",
        T::TotalGames => "Total",
        T::InstalledCount => "Installed",
        T::AvailableCount => "Available",
        T::SearchGames => "Search games…",
        T::AddGame => "Add game",
        T::AddFolder => "Add folder",
        T::Rescan => "Rescan",
        T::OpenSetup => "Open / Configure",
        T::InstalledByTool => "Installed by this tool",
        T::AddedByYou => "Added by you",
        T::NoGamesFound => "No installed games found from Steam, Epic, GOG or Xbox.",
        T::NoGamesHint => "Use Add folder to point at a game by hand.",
        T::StatusInstalled => "Installed",
        T::StatusNotInstalled => "Not installed",
        T::StatusReady => "Ready",
        T::StatusIncomplete => "Incomplete",
        T::StatusUpdate => "Update available",
        T::GameExe => "Game exe",
        T::Inspecting => "Inspecting",
        T::CollectionTitle => "Collection — games in this folder",
        T::CollectionSelected => "Selected",
        T::SelectAll => "Select all",
        T::SelectNone => "Select none",
        T::InstallMethod => "Install method",
        T::EngineReshade => "ReShade + DLSS 5",
        T::EngineReshadeDetail => "Default. Works in every supported game. Home → Add-ons → DLSS 5.",
        T::EngineOpti => "OptiScaler",
        T::EngineOptiDetail => "Dagherbou fork as dxgi.dll. Insert opens its overlay.",
        T::Recommended => "Recommended",
        T::NeuralRendering => "Neural Rendering",
        T::StableRenodx => "Stable — RenoDX DLSS 5",
        T::ExperimentalUpstream => "Experimental — Neural Upstream",
        T::Components => "Components",
        T::Install => "Install DLSS 5",
        T::InstallN => "Install DLSS 5 · {n} games",
        T::Remove => "Remove",
        T::Diagnose => "Diagnose",
        T::ConfirmRemove => "Remove this tool's files from the game?",
        T::ConfirmRemoveAll => "Remove everything this tool placed (including shared files)?",
        T::Cancel => "Cancel",
        T::FeederKnobs => "Feeder settings",
        T::Hotkeys => "Hotkeys",
        T::HotkeysHint => "Applied to every selected collection game.",
        T::WriteHotkeys => "Write hotkeys",
        T::ReloadFromDisk => "Reload from disk",
        T::Reset => "Reset",
        T::Clear => "Clear",
        T::PressKey => "Press a key… (Esc to cancel)",
        T::OverlayReshade => "ReShade overlay",
        T::ToggleNr => "Toggle neural rendering",
        T::NrScreenshot => "NR screenshot",
        T::OverlayOpti => "OptiScaler overlay",
        T::InstallLog => "Install log",
        T::WriteCfg => "Write cfg",
        T::Basics => "Basics",
        T::MotionVectors => "Motion vectors",
        T::Advanced => "Advanced",
        T::General => "General",
        T::QualityPreset => "Quality preset",
        T::FeederDefaults => "Feeder defaults",
        T::OverlayDiagnostics => "Overlay / diagnostics",
        T::Language => "Interface language",
        T::ApplyToGame => "Apply defaults to this game",
        T::ResetFeederDefaults => "Reset to Feeder defaults",
        T::SaveSettings => "Save settings",
        T::AboutTitle => "About",
        T::CommunityForkOf => "Community fork of",
        T::ForkChanges => "Changes in this fork",
        T::ForkHotkeys => "In-app hotkey management",
        T::ForkCollections => "Multi-game collection support",
        T::ForkHotkeyProp => "Collection-wide hotkey propagation",
        T::ForkNoUpdate => "Self-update disabled (upstream releases not applied)",
        T::ForkI18n => "RU / EN localization",
        T::Credits => "Credits",
        T::BuildInfo => "Build information",
        T::Version => "Version",
        T::Architecture => "Architecture",
        T::UpdateStatus => "Update status",
        T::SelfUpdateDisabled => "Disabled",
        T::X64 => "x64",
        T::Path => "Path",
        T::Yes => "Yes",
        T::No => "No",
        T::Ok => "OK",
        T::Error => "Error",
        T::Warning => "Warning",
    }
}

fn ru(key: T) -> &'static str {
    match key {
        T::Games => "Игры",
        T::Setup => "Установка",
        T::Settings => "Настройки",
        T::About => "О программе",
        T::Ready => "Готово",
        T::LeakedBuild => "Leaked Build",
        T::Support => "Поддержка",
        T::TipTitle => "Совет",
        T::TipBody => "Выберите игру на вкладке «Игры», затем нажмите «Установить» на странице «Установка». В «Настройках» можно задать Feeder-значения по умолчанию.",
        T::Dismiss => "Закрыть",
        T::YourGames => "Ваши игры",
        T::YourGamesSub => "Установка и управление DLSS 5 для найденных игр.",
        T::TotalGames => "Всего",
        T::InstalledCount => "Установлено",
        T::AvailableCount => "Доступно",
        T::SearchGames => "Поиск игр…",
        T::AddGame => "Добавить игру",
        T::AddFolder => "Добавить папку",
        T::Rescan => "Сканировать",
        T::OpenSetup => "Открыть / Настроить",
        T::InstalledByTool => "Установлено этим инструментом",
        T::AddedByYou => "Добавлено вами",
        T::NoGamesFound => "Не найдено игр Steam, Epic, GOG или Xbox.",
        T::NoGamesHint => "Используйте «Добавить папку», чтобы указать игру вручную.",
        T::StatusInstalled => "Установлено",
        T::StatusNotInstalled => "Не установлено",
        T::StatusReady => "Готово",
        T::StatusIncomplete => "Неполно",
        T::StatusUpdate => "Есть обновление",
        T::GameExe => "Exe игры",
        T::Inspecting => "Просмотр",
        T::CollectionTitle => "Коллекция — игры в этой папке",
        T::CollectionSelected => "Выбрано",
        T::SelectAll => "Выбрать все",
        T::SelectNone => "Снять все",
        T::InstallMethod => "Способ установки",
        T::EngineReshade => "ReShade + DLSS 5",
        T::EngineReshadeDetail => "По умолчанию. Подходит для всех поддерживаемых игр. Home → Add-ons → DLSS 5.",
        T::EngineOpti => "OptiScaler",
        T::EngineOptiDetail => "Форк Dagherbou как dxgi.dll. Insert открывает оверлей.",
        T::Recommended => "Рекомендуется",
        T::NeuralRendering => "Neural Rendering",
        T::StableRenodx => "Стабильный — RenoDX DLSS 5",
        T::ExperimentalUpstream => "Экспериментальный — Neural Upstream",
        T::Components => "Компоненты",
        T::Install => "Установить DLSS 5",
        T::InstallN => "Установить DLSS 5 · {n} игр",
        T::Remove => "Удалить",
        T::Diagnose => "Диагностика",
        T::ConfirmRemove => "Удалить файлы этого инструмента из игры?",
        T::ConfirmRemoveAll => "Удалить всё, что поставил этот инструмент (включая общие файлы)?",
        T::Cancel => "Отмена",
        T::FeederKnobs => "Настройки Feeder",
        T::Hotkeys => "Горячие клавиши",
        T::HotkeysHint => "Применяются ко всем выбранным играм коллекции.",
        T::WriteHotkeys => "Применить горячие клавиши",
        T::ReloadFromDisk => "Перечитать с диска",
        T::Reset => "Сброс",
        T::Clear => "Очистить",
        T::PressKey => "Нажмите клавишу… (Esc — отмена)",
        T::OverlayReshade => "Оверлей ReShade",
        T::ToggleNr => "Переключить neural rendering",
        T::NrScreenshot => "NR screenshot",
        T::OverlayOpti => "Оверлей OptiScaler",
        T::InstallLog => "Журнал установки",
        T::WriteCfg => "Записать cfg",
        T::Basics => "Основные",
        T::MotionVectors => "Векторы движения",
        T::Advanced => "Дополнительно",
        T::General => "Общие",
        T::QualityPreset => "Пресет качества",
        T::FeederDefaults => "Feeder по умолчанию",
        T::OverlayDiagnostics => "Оверлей / диагностика",
        T::Language => "Язык интерфейса",
        T::ApplyToGame => "Применить к этой игре",
        T::ResetFeederDefaults => "Сброс к Feeder defaults",
        T::SaveSettings => "Сохранить настройки",
        T::AboutTitle => "О программе",
        T::CommunityForkOf => "Community fork of",
        T::ForkChanges => "Изменения этого форка",
        T::ForkHotkeys => "Управление хоткеями в приложении",
        T::ForkCollections => "Поддержка мультиигровых коллекций",
        T::ForkHotkeyProp => "Прокидка хоткеев во все игры коллекции",
        T::ForkNoUpdate => "Автообновление отключено (релизы upstream не применяются)",
        T::ForkI18n => "Локализация RU / EN",
        T::Credits => "Благодарности",
        T::BuildInfo => "Сборка",
        T::Version => "Версия",
        T::Architecture => "Архитектура",
        T::UpdateStatus => "Обновления",
        T::SelfUpdateDisabled => "Отключено",
        T::X64 => "x64",
        T::Path => "Путь",
        T::Yes => "Да",
        T::No => "Нет",
        T::Ok => "OK",
        T::Error => "Ошибка",
        T::Warning => "Внимание",
    }
}

/// Replace `{n}` in a template (e.g. InstallN).
pub fn fmt_n(template: &str, n: usize) -> String {
    template.replace("{n}", &n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_cover_all_keys() {
        // Exhaustive match in en/ru already ensures compile-time coverage;
        // spot-check a few strings differ.
        assert_ne!(Language::Ru.t(T::Games), Language::En.t(T::Games));
        assert_eq!(Language::En.t(T::Games), "Games");
        assert_eq!(Language::Ru.t(T::Games), "Игры");
        assert!(fmt_n(Language::Ru.t(T::InstallN), 3).contains('3'));
    }

    #[test]
    fn serde_roundtrip_default_ru() {
        let l = Language::default();
        assert_eq!(l, Language::Ru);
        let s = serde_json::to_string(&l).unwrap();
        assert_eq!(s, "\"ru\"");
        let back: Language = serde_json::from_str(&s).unwrap();
        assert_eq!(back, Language::Ru);
    }
}
