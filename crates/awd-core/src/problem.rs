//! Plug and Play problem codes (`CM_PROB_*`) and what they mean for a user.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Language for user-facing texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// English.
    #[default]
    En,
    /// Russian.
    Ru,
}

/// How bad a problem code is for the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational; the device is in an expected state (disabled, ejected, ...).
    Info,
    /// The device may work after a restart or some time.
    Warning,
    /// The device does not work.
    Error,
}

/// The most useful next step for a given problem code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    /// Nothing to do.
    None,
    /// No usable driver is installed: search for one.
    FindDriver,
    /// A driver exists but is broken: reinstall it.
    ReinstallDriver,
    /// The driver loads but misbehaves: try a different (newer) version.
    UpdateDriver,
    /// The device or its service was disabled on purpose.
    Enable,
    /// A restart should clear the state.
    Restart,
    /// Likely a hardware, firmware or connection issue.
    CheckHardware,
    /// Needs manual investigation (resources, registry, dependencies).
    Investigate,
}

/// A Device Manager problem code, e.g. `28` for "drivers not installed".
///
/// The numeric value is what `CM_Get_DevNode_Status` and
/// `DEVPKEY_Device_ProblemCode` report; this type adds names, descriptions,
/// severity and a suggested action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProblemCode(pub u32);

struct Info {
    name: &'static str,
    en: &'static str,
    ru: &'static str,
    severity: Severity,
    action: SuggestedAction,
}

#[allow(missing_docs)]
impl ProblemCode {
    pub const NOT_CONFIGURED: Self = Self(1);
    pub const FAILED_START: Self = Self(10);
    pub const NEED_RESTART: Self = Self(14);
    pub const REINSTALL: Self = Self(18);
    pub const DISABLED: Self = Self(22);
    pub const DEVICE_NOT_THERE: Self = Self(24);
    pub const FAILED_INSTALL: Self = Self(28);
    pub const DISABLED_SERVICE: Self = Self(32);
    pub const FAILED_DRIVER_ENTRY: Self = Self(37);
    pub const DRIVER_FAILED_LOAD: Self = Self(39);
    pub const FAILED_POST_START: Self = Self(43);
    pub const PHANTOM: Self = Self(45);
    pub const DRIVER_BLOCKED: Self = Self(48);
    pub const UNSIGNED_DRIVER: Self = Self(52);
    pub const NEED_CLASS_CONFIG: Self = Self(56);
}

impl ProblemCode {
    /// The numeric code.
    pub const fn code(self) -> u32 {
        self.0
    }

    /// Symbolic name without the `CM_PROB_` prefix, e.g. `FAILED_INSTALL`.
    pub fn name(self) -> &'static str {
        self.info().name
    }

    /// One-sentence explanation for the user.
    pub fn describe(self, lang: Lang) -> &'static str {
        let info = self.info();
        match lang {
            Lang::En => info.en,
            Lang::Ru => info.ru,
        }
    }

    /// How serious the problem is.
    pub fn severity(self) -> Severity {
        self.info().severity
    }

    /// What the user (or this tool) should do about it.
    pub fn action(self) -> SuggestedAction {
        self.info().action
    }

    /// `true` when the fix is a driver: find, reinstall or update one.
    pub fn is_driver_issue(self) -> bool {
        matches!(
            self.action(),
            SuggestedAction::FindDriver
                | SuggestedAction::ReinstallDriver
                | SuggestedAction::UpdateDriver
        )
    }

    /// `true` when the device has no usable driver at all (codes 1, 28, 52).
    pub fn needs_driver(self) -> bool {
        self.action() == SuggestedAction::FindDriver
    }

    /// `true` when the problem code is one this crate knows by name.
    pub fn is_known(self) -> bool {
        (1..=57).contains(&self.0)
    }

    fn info(self) -> Info {
        use Severity::{Error, Info as InfoSev, Warning};
        use SuggestedAction as A;
        macro_rules! info {
            ($name:literal, $sev:expr, $act:expr, $en:literal, $ru:literal) => {
                Info {
                    name: $name,
                    en: $en,
                    ru: $ru,
                    severity: $sev,
                    action: $act,
                }
            };
        }
        match self.0 {
            1 => info!(
                "NOT_CONFIGURED",
                Error,
                A::FindDriver,
                "The device is not configured correctly: no driver is installed.",
                "Устройство настроено неправильно: драйвер не установлен."
            ),
            2 => info!(
                "DEVLOADER_FAILED",
                Error,
                A::ReinstallDriver,
                "Windows could not load the driver for this device.",
                "Windows не удалось загрузить драйвер устройства."
            ),
            3 => info!(
                "OUT_OF_MEMORY",
                Error,
                A::ReinstallDriver,
                "The driver may be corrupted, or the system is low on memory or resources.",
                "Драйвер повреждён либо системе не хватает памяти или ресурсов."
            ),
            4 => info!(
                "ENTRY_IS_WRONG_TYPE",
                Error,
                A::ReinstallDriver,
                "The device is not working properly; a driver or registry entry may be corrupted.",
                "Устройство работает неправильно: возможно, повреждён драйвер или реестр."
            ),
            5 => info!(
                "LACKED_ARBITRATOR",
                Error,
                A::UpdateDriver,
                "The driver needs a resource that Windows cannot manage.",
                "Драйверу нужен ресурс, которым Windows не может управлять."
            ),
            6 => info!(
                "BOOT_CONFIG_CONFLICT",
                Error,
                A::Investigate,
                "The boot configuration conflicts with other devices.",
                "Загрузочная конфигурация конфликтует с другими устройствами."
            ),
            7 => info!(
                "FAILED_FILTER",
                Error,
                A::ReinstallDriver,
                "The device cannot filter.",
                "Устройству не удалось выполнить фильтрацию."
            ),
            8 => info!(
                "DEVLOADER_NOT_FOUND",
                Error,
                A::ReinstallDriver,
                "The driver loader for the device is missing.",
                "Загрузчик драйвера устройства не найден."
            ),
            9 => info!(
                "INVALID_DATA",
                Error,
                A::CheckHardware,
                "Windows cannot identify this hardware: it does not report a valid hardware ID.",
                "Windows не может определить это оборудование: нет корректного идентификатора."
            ),
            10 => info!(
                "FAILED_START",
                Error,
                A::UpdateDriver,
                "The device cannot start.",
                "Запуск устройства невозможен."
            ),
            11 => info!(
                "LIAR",
                Error,
                A::CheckHardware,
                "The device failed.",
                "Сбой устройства."
            ),
            12 => info!(
                "NORMAL_CONFLICT",
                Error,
                A::Investigate,
                "The device cannot find enough free resources to use.",
                "Устройству не хватает свободных ресурсов."
            ),
            13 => info!(
                "NOT_VERIFIED",
                Error,
                A::Investigate,
                "Windows cannot verify the device's resources.",
                "Windows не может проверить ресурсы устройства."
            ),
            14 => info!(
                "NEED_RESTART",
                Warning,
                A::Restart,
                "The device will work properly after the computer is restarted.",
                "Устройство заработает после перезагрузки компьютера."
            ),
            15 => info!(
                "REENUMERATION",
                Warning,
                A::Restart,
                "The device is not working properly because of a possible re-enumeration problem.",
                "Устройство работает неправильно: возможна проблема повторного перечисления."
            ),
            16 => info!(
                "PARTIAL_LOG_CONF",
                Error,
                A::Investigate,
                "Windows cannot identify all the resources the device uses.",
                "Windows не может определить все ресурсы, используемые устройством."
            ),
            17 => info!(
                "UNKNOWN_RESOURCE",
                Error,
                A::ReinstallDriver,
                "The device is asking for an unknown resource type.",
                "Устройство запрашивает неизвестный тип ресурса."
            ),
            18 => info!(
                "REINSTALL",
                Error,
                A::ReinstallDriver,
                "The drivers for this device need to be reinstalled.",
                "Драйверы этого устройства нужно переустановить."
            ),
            19 => info!(
                "REGISTRY",
                Error,
                A::ReinstallDriver,
                "The device's configuration information in the registry is incomplete or damaged.",
                "Сведения о конфигурации устройства в реестре повреждены или неполны."
            ),
            20 => info!(
                "VXDLDR",
                Error,
                A::ReinstallDriver,
                "Windows could not load one of the drivers for this device.",
                "Windows не удалось загрузить один из драйверов устройства."
            ),
            21 => info!(
                "WILL_BE_REMOVED",
                InfoSev,
                A::Restart,
                "Windows is removing this device.",
                "Windows удаляет это устройство."
            ),
            22 => info!(
                "DISABLED",
                InfoSev,
                A::Enable,
                "The device is disabled.",
                "Устройство отключено."
            ),
            23 => info!(
                "DEVLOADER_NOT_READY",
                Warning,
                A::Restart,
                "The device loader is not ready.",
                "Загрузчик устройства не готов."
            ),
            24 => info!(
                "DEVICE_NOT_THERE",
                Error,
                A::CheckHardware,
                "The device is not present, is not working properly, or does not have all its drivers installed.",
                "Устройство отсутствует, работает неправильно или установлены не все драйверы."
            ),
            25 => info!(
                "MOVED",
                Warning,
                A::Restart,
                "Windows is still setting up this device.",
                "Windows ещё настраивает это устройство."
            ),
            26 => info!(
                "TOO_EARLY",
                Warning,
                A::Restart,
                "Windows is still setting up this device.",
                "Windows ещё настраивает это устройство."
            ),
            27 => info!(
                "NO_VALID_LOG_CONF",
                Error,
                A::Investigate,
                "The device does not have a valid logical configuration.",
                "У устройства нет корректной логической конфигурации."
            ),
            28 => info!(
                "FAILED_INSTALL",
                Error,
                A::FindDriver,
                "The drivers for this device are not installed.",
                "Драйверы для этого устройства не установлены."
            ),
            29 => info!(
                "HARDWARE_DISABLED",
                Warning,
                A::CheckHardware,
                "The device is disabled because its firmware did not give it the required resources.",
                "Устройство отключено: прошивка не выделила ему необходимые ресурсы."
            ),
            30 => info!(
                "CANT_SHARE_IRQ",
                Error,
                A::Investigate,
                "The device is using an IRQ that another device is using.",
                "Устройство использует IRQ, занятый другим устройством."
            ),
            31 => info!(
                "FAILED_ADD",
                Error,
                A::ReinstallDriver,
                "Windows cannot load the drivers required for this device.",
                "Windows не может загрузить необходимые устройству драйверы."
            ),
            32 => info!(
                "DISABLED_SERVICE",
                Warning,
                A::Enable,
                "A driver service for this device has been disabled.",
                "Служба драйвера этого устройства отключена."
            ),
            33 => info!(
                "TRANSLATION_FAILED",
                Error,
                A::CheckHardware,
                "Windows cannot determine which resources are required for this device.",
                "Windows не может определить, какие ресурсы нужны устройству."
            ),
            34 => info!(
                "NO_SOFTCONFIG",
                Error,
                A::Investigate,
                "Windows cannot determine the settings for this device.",
                "Windows не может определить параметры устройства."
            ),
            35 => info!(
                "BIOS_TABLE",
                Error,
                A::CheckHardware,
                "The system firmware does not include enough information to configure this device.",
                "Прошивка компьютера не содержит достаточно сведений для настройки устройства."
            ),
            36 => info!(
                "IRQ_TRANSLATION_FAILED",
                Error,
                A::CheckHardware,
                "The device is requesting a PCI interrupt but is configured for an ISA interrupt, or vice versa.",
                "Устройство запрашивает прерывание PCI, но настроено на ISA, или наоборот."
            ),
            37 => info!(
                "FAILED_DRIVER_ENTRY",
                Error,
                A::UpdateDriver,
                "Windows cannot initialize the device driver for this hardware.",
                "Windows не может инициализировать драйвер устройства."
            ),
            38 => info!(
                "DRIVER_FAILED_PRIOR_UNLOAD",
                Error,
                A::Restart,
                "Windows cannot load the driver because a previous instance is still in memory.",
                "Предыдущий экземпляр драйвера всё ещё находится в памяти."
            ),
            39 => info!(
                "DRIVER_FAILED_LOAD",
                Error,
                A::ReinstallDriver,
                "Windows cannot load the driver for this device; it may be corrupted or missing.",
                "Не удалось загрузить драйвер устройства: он повреждён или отсутствует."
            ),
            40 => info!(
                "DRIVER_SERVICE_KEY_INVALID",
                Error,
                A::ReinstallDriver,
                "Information about the driver's service key in the registry is missing or invalid.",
                "Сведения о ключе службы драйвера в реестре отсутствуют или неверны."
            ),
            41 => info!(
                "LEGACY_SERVICE_NO_DEVICES",
                Warning,
                A::CheckHardware,
                "Windows loaded the driver but cannot find the hardware device.",
                "Драйвер загружен, но устройство не найдено."
            ),
            42 => info!(
                "DUPLICATE_DEVICE",
                Error,
                A::Restart,
                "A duplicate device is already running in the system.",
                "В системе уже работает дубликат этого устройства."
            ),
            43 => info!(
                "FAILED_POST_START",
                Error,
                A::UpdateDriver,
                "Windows has stopped this device because it has reported problems.",
                "Windows остановила устройство, так как оно сообщило о неполадках."
            ),
            44 => info!(
                "HALTED",
                Warning,
                A::Restart,
                "An application or service has shut down this device.",
                "Приложение или служба завершили работу устройства."
            ),
            45 => info!(
                "PHANTOM",
                InfoSev,
                A::None,
                "The device is not connected to the computer.",
                "Устройство не подключено к компьютеру."
            ),
            46 => info!(
                "SYSTEM_SHUTDOWN",
                InfoSev,
                A::None,
                "Windows is shutting down; the device is not available.",
                "Windows завершает работу, устройство недоступно."
            ),
            47 => info!(
                "HELD_FOR_EJECT",
                InfoSev,
                A::None,
                "The device has been prepared for safe removal but has not been removed.",
                "Устройство подготовлено к безопасному извлечению, но не извлечено."
            ),
            48 => info!(
                "DRIVER_BLOCKED",
                Error,
                A::UpdateDriver,
                "The driver is blocked from loading because it is known to cause problems.",
                "Загрузка драйвера заблокирована: он известен проблемами с Windows."
            ),
            49 => info!(
                "REGISTRY_TOO_LARGE",
                Error,
                A::Investigate,
                "The system registry hive is too large.",
                "Системный куст реестра слишком велик."
            ),
            50 => info!(
                "SETPROPERTIES_FAILED",
                Error,
                A::ReinstallDriver,
                "Windows cannot apply all of the properties for this device.",
                "Windows не может применить все свойства устройства."
            ),
            51 => info!(
                "WAITING_ON_DEPENDENCY",
                Warning,
                A::Investigate,
                "The device is waiting on another device to start.",
                "Устройство ожидает запуска другого устройства."
            ),
            52 => info!(
                "UNSIGNED_DRIVER",
                Error,
                A::FindDriver,
                "Windows cannot verify the digital signature of the drivers for this device.",
                "Windows не может проверить цифровую подпись драйверов устройства."
            ),
            53 => info!(
                "USED_BY_DEBUGGER",
                InfoSev,
                A::None,
                "The device is reserved for the Windows kernel debugger.",
                "Устройство зарезервировано отладчиком ядра Windows."
            ),
            54 => info!(
                "DEVICE_RESET",
                Warning,
                A::None,
                "The device has failed and is being reset.",
                "Устройство дало сбой и перезапускается."
            ),
            55 => info!(
                "CONSOLE_LOCKED",
                Warning,
                A::None,
                "Windows cannot start the device while the console is locked.",
                "Windows не может запустить устройство при заблокированной консоли."
            ),
            56 => info!(
                "NEED_CLASS_CONFIG",
                Warning,
                A::ReinstallDriver,
                "The device requires further installation.",
                "Устройство требует дополнительной установки."
            ),
            57 => info!(
                "GUEST_ASSIGNMENT_FAILED",
                Warning,
                A::Investigate,
                "Windows could not assign this device to a virtual machine.",
                "Не удалось назначить устройство виртуальной машине."
            ),
            _ => info!(
                "UNKNOWN",
                Error,
                A::Investigate,
                "Unknown problem code.",
                "Неизвестный код проблемы."
            ),
        }
    }
}

impl fmt::Display for ProblemCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.0, self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_code_has_texts_in_both_languages() {
        for code in 1..=57 {
            let p = ProblemCode(code);
            assert!(p.is_known());
            assert_ne!(p.name(), "UNKNOWN", "code {code}");
            assert!(!p.describe(Lang::En).is_empty());
            assert!(!p.describe(Lang::Ru).is_empty());
            assert_ne!(p.describe(Lang::En), p.describe(Lang::Ru), "code {code}");
        }
        assert_eq!(ProblemCode(0).name(), "UNKNOWN");
        assert_eq!(ProblemCode(999).name(), "UNKNOWN");
        assert!(!ProblemCode(999).is_known());
    }

    #[test]
    fn driver_related_codes_are_classified() {
        assert!(ProblemCode::FAILED_INSTALL.needs_driver());
        assert!(ProblemCode::NOT_CONFIGURED.needs_driver());
        assert!(ProblemCode::UNSIGNED_DRIVER.needs_driver());
        assert!(!ProblemCode::DISABLED.needs_driver());
        assert!(ProblemCode::FAILED_POST_START.is_driver_issue());
        assert!(ProblemCode::REINSTALL.is_driver_issue());
        assert!(!ProblemCode::NEED_RESTART.is_driver_issue());
        assert_eq!(ProblemCode::DISABLED.severity(), Severity::Info);
        assert_eq!(ProblemCode::DISABLED.action(), SuggestedAction::Enable);
    }

    #[test]
    fn display_includes_code_and_name() {
        assert_eq!(ProblemCode(28).to_string(), "28 (FAILED_INSTALL)");
    }

    #[test]
    fn serializes_as_bare_number() {
        assert_eq!(serde_json::to_string(&ProblemCode(22)).unwrap(), "22");
        let p: ProblemCode = serde_json::from_str("28").unwrap();
        assert_eq!(p, ProblemCode::FAILED_INSTALL);
    }
}
