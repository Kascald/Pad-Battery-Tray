//! XInput 원시값을 표시 상태로 바꾸고, 순환·알림 판단을 한다. Win32에 의존하지 않는 순수 로직이다.

use std::fmt::Write as _;

pub const SLOTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryType {
    Disconnected,
    Wired,
    Alkaline,
    NiMh,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryLevel {
    Empty,
    Low,
    Medium,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawBattery {
    pub kind: BatteryType,
    pub level: BatteryLevel,
}

impl RawBattery {
    pub const DISCONNECTED: Self = Self { kind: BatteryType::Disconnected, level: BatteryLevel::Empty };

    pub const fn new(kind: BatteryType, level: BatteryLevel) -> Self {
        Self { kind, level }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Display {
    NoConnection,
    Pending,
    Wired,
    Empty,
    Low,
    Medium,
    Full,
}

impl Display {
    pub fn from_raw(raw: RawBattery) -> Self {
        match raw.kind {
            BatteryType::Disconnected => Self::NoConnection,
            BatteryType::Wired => Self::Wired,
            BatteryType::Unknown => Self::Pending,
            BatteryType::Alkaline | BatteryType::NiMh => match raw.level {
                BatteryLevel::Empty => Self::Empty,
                BatteryLevel::Low => Self::Low,
                BatteryLevel::Medium => Self::Medium,
                BatteryLevel::Full => Self::Full,
            },
        }
    }

    pub fn is_connected(self) -> bool {
        self != Self::NoConnection
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NoConnection => "연결 없음",
            Self::Pending => "대기",
            Self::Wired => "유선",
            Self::Empty => "비어 있음",
            Self::Low => "낮음",
            Self::Medium => "보통",
            Self::Full => "가득",
        }
    }

    fn meets(self, threshold: Threshold) -> bool {
        match threshold {
            Threshold::Low => matches!(self, Self::Low | Self::Empty),
            Threshold::Empty => self == Self::Empty,
        }
    }

    /// LOW/MEDIUM 경계의 오르내림으로 알림이 반복되지 않도록, 가득 또는 유선에서만 알림을 다시 켠다.
    fn rearms(self) -> bool {
        matches!(self, Self::Full | Self::Wired)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Threshold {
    #[default]
    Low,
    Empty,
}

pub type Snapshot = [Display; SLOTS];

pub fn snapshot(raws: [RawBattery; SLOTS]) -> Snapshot {
    raws.map(Display::from_raw)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alert {
    pub slot: usize,
    pub state: Display,
}

/// 슬롯별로 알림을 한 번만 내기 위한 상태. 시작 시 모두 무장되어 있어 이미 낮은 패드는 한 번 알린다.
#[derive(Debug)]
pub struct AlertTracker {
    armed: [bool; SLOTS],
}

impl Default for AlertTracker {
    fn default() -> Self {
        Self { armed: [true; SLOTS] }
    }
}

impl AlertTracker {
    pub fn update(&mut self, states: &Snapshot, threshold: Threshold) -> Vec<Alert> {
        let mut alerts = Vec::new();
        for (slot, &state) in states.iter().enumerate() {
            if state.rearms() {
                self.armed[slot] = true;
            } else if state.meets(threshold) && self.armed[slot] {
                self.armed[slot] = false;
                alerts.push(Alert { slot, state });
            }
        }
        alerts
    }
}

/// 아이콘에 보여 줄 슬롯 순번. 연결된 슬롯만 돈다.
#[derive(Debug, Default)]
pub struct Rotation {
    current: Option<usize>,
}

impl Rotation {
    /// `advance`가 참이면 다음 연결 슬롯으로 넘어가고, 거짓이면 현재 슬롯이 유효한 동안 유지한다.
    pub fn select(&mut self, states: &Snapshot, advance: bool) -> Option<usize> {
        let start = match self.current {
            Some(cur) if advance => cur + 1,
            Some(cur) => cur,
            None => 0,
        };
        self.current = (0..SLOTS)
            .map(|i| (start + i) % SLOTS)
            .find(|&slot| states[slot].is_connected());
        self.current
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayView {
    pub icon: Display,
    /// 2대 이상 연결됐을 때만 1부터 시작하는 슬롯 번호를 아이콘에 겹친다.
    pub badge: Option<u8>,
    pub tooltip: String,
}

pub fn tray_view(states: &Snapshot, current: Option<usize>) -> TrayView {
    let connected = states.iter().filter(|s| s.is_connected()).count();
    let Some(slot) = current.filter(|&s| states[s].is_connected()) else {
        return TrayView { icon: Display::NoConnection, badge: None, tooltip: "연결된 패드 없음".into() };
    };

    let mut tooltip = String::new();
    for (i, s) in states.iter().enumerate().filter(|(_, s)| s.is_connected()) {
        if !tooltip.is_empty() {
            tooltip.push('\n');
        }
        let _ = write!(tooltip, "P{}: {}", i + 1, s.label());
    }

    TrayView {
        icon: states[slot],
        badge: (connected >= 2).then_some(slot as u8 + 1),
        tooltip,
    }
}

/// 한 번의 조회 결과에 대한 알림 제목과 본문.
pub fn alert_message(alerts: &[Alert]) -> Option<(String, String)> {
    let worst = alerts.iter().any(|a| a.state == Display::Empty);
    let title = if worst { "패드 배터리 없음" } else { "패드 배터리 낮음" };
    let body = alerts
        .iter()
        .map(|a| format!("P{}: {}", a.slot + 1, a.state.label()))
        .collect::<Vec<_>>()
        .join(" · ");
    (!alerts.is_empty()).then(|| (title.to_string(), format!("{body}. 충전하거나 배터리를 교체하세요.")))
}

/// 조회 한 번을 처리한다. 앱의 갱신 루프가 이 함수만 호출한다.
#[derive(Debug, Default)]
pub struct Monitor {
    tracker: AlertTracker,
    rotation: Rotation,
}

impl Monitor {
    pub fn refresh(
        &mut self,
        raws: [RawBattery; SLOTS],
        threshold: Threshold,
        advance: bool,
    ) -> (TrayView, Vec<Alert>) {
        let states = snapshot(raws);
        let alerts = self.tracker.update(&states, threshold);
        let current = self.rotation.select(&states, advance);
        (tray_view(&states, current), alerts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BatteryLevel as L;
    use BatteryType as T;

    const OFF: RawBattery = RawBattery::DISCONNECTED;

    fn pad(level: BatteryLevel) -> RawBattery {
        RawBattery::new(T::NiMh, level)
    }

    fn wired() -> RawBattery {
        RawBattery::new(T::Wired, L::Full)
    }

    #[test]
    fn maps_raw_values() {
        assert_eq!(Display::from_raw(OFF), Display::NoConnection);
        assert_eq!(Display::from_raw(wired()), Display::Wired);
        assert_eq!(Display::from_raw(RawBattery::new(T::Unknown, L::Full)), Display::Pending);
        assert_eq!(Display::from_raw(RawBattery::new(T::Alkaline, L::Empty)), Display::Empty);
        assert_eq!(Display::from_raw(pad(L::Low)), Display::Low);
        assert_eq!(Display::from_raw(pad(L::Medium)), Display::Medium);
        assert_eq!(Display::from_raw(pad(L::Full)), Display::Full);
    }

    #[test]
    fn alerts_once_when_dropping_to_low() {
        let mut t = AlertTracker::default();
        let s = |l| snapshot([pad(l), OFF, OFF, OFF]);
        assert!(t.update(&s(L::Medium), Threshold::Low).is_empty());
        assert_eq!(t.update(&s(L::Low), Threshold::Low), vec![Alert { slot: 0, state: Display::Low }]);
        assert!(t.update(&s(L::Low), Threshold::Low).is_empty());
        assert!(t.update(&s(L::Empty), Threshold::Low).is_empty());
    }

    #[test]
    fn low_medium_flapping_does_not_realert() {
        let mut t = AlertTracker::default();
        let s = |l| snapshot([pad(l), OFF, OFF, OFF]);
        assert_eq!(t.update(&s(L::Low), Threshold::Low).len(), 1);
        assert!(t.update(&s(L::Medium), Threshold::Low).is_empty());
        assert!(t.update(&s(L::Low), Threshold::Low).is_empty());
    }

    #[test]
    fn rearms_after_full_or_wired() {
        let mut t = AlertTracker::default();
        let s = |r| snapshot([r, OFF, OFF, OFF]);
        assert_eq!(t.update(&s(pad(L::Low)), Threshold::Low).len(), 1);
        t.update(&s(pad(L::Full)), Threshold::Low);
        assert_eq!(t.update(&s(pad(L::Low)), Threshold::Low).len(), 1);
        t.update(&s(wired()), Threshold::Low);
        assert_eq!(t.update(&s(pad(L::Low)), Threshold::Low).len(), 1);
    }

    #[test]
    fn alerts_on_startup_when_already_low() {
        let mut t = AlertTracker::default();
        let alerts = t.update(&snapshot([OFF, pad(L::Empty), OFF, OFF]), Threshold::Low);
        assert_eq!(alerts, vec![Alert { slot: 1, state: Display::Empty }]);
    }

    #[test]
    fn empty_threshold_ignores_low() {
        let mut t = AlertTracker::default();
        let s = |l| snapshot([pad(l), OFF, OFF, OFF]);
        assert!(t.update(&s(L::Low), Threshold::Empty).is_empty());
        assert_eq!(t.update(&s(L::Empty), Threshold::Empty).len(), 1);
    }

    #[test]
    fn disconnect_keeps_alert_disarmed() {
        let mut t = AlertTracker::default();
        let s = |r| snapshot([r, OFF, OFF, OFF]);
        assert_eq!(t.update(&s(pad(L::Low)), Threshold::Low).len(), 1);
        t.update(&s(OFF), Threshold::Low);
        assert!(t.update(&s(pad(L::Low)), Threshold::Low).is_empty());
    }

    #[test]
    fn rotation_cycles_connected_slots_only() {
        let mut r = Rotation::default();
        let s = snapshot([pad(L::Full), OFF, wired(), OFF]);
        assert_eq!(r.select(&s, true), Some(0));
        assert_eq!(r.select(&s, true), Some(2));
        assert_eq!(r.select(&s, true), Some(0));
        assert_eq!(r.select(&s, false), Some(0));
    }

    #[test]
    fn rotation_moves_off_disconnected_slot_without_advancing() {
        let mut r = Rotation::default();
        r.select(&snapshot([OFF, pad(L::Full), OFF, wired()]), true);
        assert_eq!(r.select(&snapshot([OFF, OFF, OFF, wired()]), false), Some(3));
        assert_eq!(r.select(&snapshot([OFF; SLOTS]), false), None);
    }

    #[test]
    fn view_without_pads() {
        let v = tray_view(&snapshot([OFF; SLOTS]), None);
        assert_eq!(v.icon, Display::NoConnection);
        assert_eq!(v.badge, None);
        assert_eq!(v.tooltip, "연결된 패드 없음");
    }

    #[test]
    fn view_single_pad_has_no_badge() {
        let v = tray_view(&snapshot([OFF, pad(L::Medium), OFF, OFF]), Some(1));
        assert_eq!(v.icon, Display::Medium);
        assert_eq!(v.badge, None);
        assert_eq!(v.tooltip, "P2: 보통");
    }

    #[test]
    fn view_multiple_pads_shows_badge_and_all_lines() {
        let v = tray_view(&snapshot([pad(L::Medium), wired(), OFF, OFF]), Some(1));
        assert_eq!(v.icon, Display::Wired);
        assert_eq!(v.badge, Some(2));
        assert_eq!(v.tooltip, "P1: 보통\nP2: 유선");
    }

    #[test]
    fn view_fits_tooltip_limit() {
        let all = [pad(L::Empty), pad(L::Empty), pad(L::Empty), pad(L::Empty)];
        let v = tray_view(&snapshot(all), Some(0));
        assert!(v.tooltip.encode_utf16().count() < 128);
    }

    #[test]
    fn alert_message_combines_slots() {
        assert_eq!(alert_message(&[]), None);
        let (title, body) = alert_message(&[
            Alert { slot: 0, state: Display::Low },
            Alert { slot: 2, state: Display::Empty },
        ])
        .unwrap();
        assert_eq!(title, "패드 배터리 없음");
        assert!(body.starts_with("P1: 낮음 · P3: 비어 있음"));
    }

    #[test]
    fn monitor_ties_it_together() {
        let mut m = Monitor::default();
        let raws = [pad(L::Low), wired(), OFF, OFF];
        let (v, alerts) = m.refresh(raws, Threshold::Low, true);
        assert_eq!((v.icon, v.badge), (Display::Low, Some(1)));
        assert_eq!(alerts.len(), 1);
        let (v, alerts) = m.refresh(raws, Threshold::Low, true);
        assert_eq!((v.icon, v.badge), (Display::Wired, Some(2)));
        assert!(alerts.is_empty());
    }
}
