//! XInput 배터리 조회. 트레이트 뒤에 두어 테스트에서 가짜 구현으로 바꾼다.

use crate::state::{RawBattery, SLOTS};

pub trait BatterySource {
    fn read(&self, slot: u32) -> RawBattery;

    fn read_all(&self) -> [RawBattery; SLOTS] {
        std::array::from_fn(|slot| self.read(slot as u32))
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
