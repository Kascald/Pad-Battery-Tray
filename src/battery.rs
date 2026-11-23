//! XInput 배터리 조회. 트레이트 뒤에 두어 테스트에서 가짜 구현으로 바꾼다.

use crate::state::{BatteryLevel, BatteryType, RawBattery, SLOTS};
use std::cell::Cell;

pub trait BatterySource {
    fn read(&self, slot: u32) -> RawBattery;

    fn read_all(&self) -> [RawBattery; SLOTS] {
        std::array::from_fn(|slot| self.read(slot as u32))
    }
}

/// `--demo` 실행용. 패드 없이 모든 아이콘·번호·알림을 보여 주도록 정해진 장면을 차례로 돌려준다.
pub struct DemoSource {
    step: Cell<usize>,
}

impl DemoSource {
    pub const fn new() -> Self {
        Self { step: Cell::new(0) }
    }
}

const fn p(kind: BatteryType, level: BatteryLevel) -> RawBattery {
    RawBattery::new(kind, level)
}

const OFF: RawBattery = RawBattery::DISCONNECTED;
const PENDING: RawBattery = p(BatteryType::Unknown, BatteryLevel::Empty);
const WIRED: RawBattery = p(BatteryType::Wired, BatteryLevel::Full);
const FULL: RawBattery = p(BatteryType::Alkaline, BatteryLevel::Full);
const MEDIUM: RawBattery = p(BatteryType::Alkaline, BatteryLevel::Medium);
const LOW: RawBattery = p(BatteryType::Alkaline, BatteryLevel::Low);
const EMPTY: RawBattery = p(BatteryType::Alkaline, BatteryLevel::Empty);

/// 1대로 7가지 아이콘을 보여 준 뒤 2대로 번호 순환과 두 번째 패드 알림을 보여 준다.
/// P1은 가득을 거쳐 알림이 다시 켜지고, P2는 유선을 거쳐 다시 켜지므로 반복할 때마다 알림이 뜬다.
pub const DEMO_SCENES: [[RawBattery; SLOTS]; 13] = [
    [OFF, OFF, OFF, OFF],
    [PENDING, OFF, OFF, OFF],
    [WIRED, OFF, OFF, OFF],
    [FULL, OFF, OFF, OFF],
    [MEDIUM, OFF, OFF, OFF],
    [LOW, OFF, OFF, OFF],
    [EMPTY, OFF, OFF, OFF],
    [FULL, WIRED, OFF, OFF],
    [FULL, WIRED, OFF, OFF],
    [FULL, LOW, OFF, OFF],
    [FULL, LOW, OFF, OFF],
    [FULL, LOW, OFF, OFF],
    [FULL, LOW, OFF, OFF],
];

impl BatterySource for DemoSource {
    fn read(&self, slot: u32) -> RawBattery {
        DEMO_SCENES[self.step.get()][slot as usize]
    }

    /// 한 번 조회할 때마다 다음 장면으로 넘어간다.
    fn read_all(&self) -> [RawBattery; SLOTS] {
        let i = self.step.get();
        self.step.set((i + 1) % DEMO_SCENES.len());
        DEMO_SCENES[i]
    }
}

#[cfg(windows)]
pub use xinput::XInput;

#[cfg(windows)]
mod xinput {
    use super::BatterySource;
    use crate::state::{BatteryLevel, BatteryType, RawBattery};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::UI::Input::XboxController::*;

    pub struct XInput;

    impl BatterySource for XInput {
        fn read(&self, slot: u32) -> RawBattery {
            let mut info = XINPUT_BATTERY_INFORMATION::default();
            // 미연결 슬롯은 ERROR_DEVICE_NOT_CONNECTED를 돌려준다
            let rc = unsafe { XInputGetBatteryInformation(slot, BATTERY_DEVTYPE_GAMEPAD, &mut info) };
            if rc != ERROR_SUCCESS.0 {
                return RawBattery::DISCONNECTED;
            }
            let kind = match info.BatteryType {
                BATTERY_TYPE_DISCONNECTED => BatteryType::Disconnected,
                BATTERY_TYPE_WIRED => BatteryType::Wired,
                BATTERY_TYPE_ALKALINE => BatteryType::Alkaline,
                BATTERY_TYPE_NIMH => BatteryType::NiMh,
                _ => BatteryType::Unknown,
            };
            let level = match info.BatteryLevel {
                BATTERY_LEVEL_EMPTY => BatteryLevel::Empty,
                BATTERY_LEVEL_LOW => BatteryLevel::Low,
                BATTERY_LEVEL_MEDIUM => BatteryLevel::Medium,
                _ => BatteryLevel::Full,
            };
            RawBattery::new(kind, level)
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::BatterySource;
    use crate::state::{RawBattery, SLOTS};
    use std::cell::Cell;

    pub struct FakeSource(pub Cell<[RawBattery; SLOTS]>);

    impl BatterySource for FakeSource {
        fn read(&self, slot: u32) -> RawBattery {
            self.0.get()[slot as usize]
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::state::*;

        #[test]
        fn monitor_with_fake_source() {
            let src = FakeSource(Cell::new([RawBattery::DISCONNECTED; SLOTS]));
            let mut m = Monitor::default();
            let (v, _) = m.refresh(src.read_all(), Threshold::Low, true);
            assert_eq!(v.icon, Display::NoConnection);

            let mut raws = [RawBattery::DISCONNECTED; SLOTS];
            raws[3] = RawBattery::new(BatteryType::Alkaline, BatteryLevel::Low);
            src.0.set(raws);
            let (v, alerts) = m.refresh(src.read_all(), Threshold::Low, false);
            assert_eq!(v.icon, Display::Low);
            assert_eq!(v.tooltip, "P4: 낮음");
            assert_eq!(alerts, vec![Alert { slot: 3, state: Display::Low }]);
        }
    }
}

#[cfg(test)]
mod demo_tests {
    use super::*;
    use crate::state::*;

    #[test]
    fn demo_loops_through_scenes() {
        let src = DemoSource::new();
        let first = src.read_all();
        for _ in 1..DEMO_SCENES.len() {
            src.read_all();
        }
        assert_eq!(src.read_all(), first);
    }

    #[test]
    fn demo_shows_every_icon_badge_and_alert_each_loop() {
        let src = DemoSource::new();
        let mut m = Monitor::default();
        for round in 0..2 {
            let mut icons = Vec::new();
            let mut badges = Vec::new();
            let mut alerts = Vec::new();
            for _ in 0..DEMO_SCENES.len() {
                let (v, a) = m.refresh(src.read_all(), Threshold::Low, true);
                if !icons.contains(&v.icon) {
                    icons.push(v.icon);
                }
                badges.extend(v.badge);
                alerts.extend(a);
            }
            assert_eq!(icons.len(), 7, "round {round}: {icons:?}");
            assert!(badges.contains(&1) && badges.contains(&2), "round {round}");
            assert_eq!(
                alerts,
                vec![Alert { slot: 0, state: Display::Low }, Alert { slot: 1, state: Display::Low }],
                "round {round}"
            );
        }
    }
}
