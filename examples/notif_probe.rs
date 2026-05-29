// 探针：对比不同 handle/连接生命周期是否触发 GNOME 秒关（看 dbus reason=2）
use std::thread::sleep;
use std::time::Duration;
use notify_rust::{Notification, Urgency, Timeout};

fn base(summary: &str) -> Notification {
    let mut n = Notification::new();
    n.summary(summary).body("probe").appname("Lifetime")
        .icon("dialog-information").timeout(Timeout::Milliseconds(5000));
    n
}

fn main() {
    // P1：发完立刻丢 handle，进程继续存活（= 当前 app 行为）
    if let Ok(h) = base("P1-drop-immediate").show() { drop(h); }
    sleep(Duration::from_millis(1500));

    // P2：持有 handle 1.5s 再丢
    if let Ok(h) = base("P2-hold-1500ms").show() {
        sleep(Duration::from_millis(1500));
        drop(h);
    }
    sleep(Duration::from_millis(500));

    // P3：Critical + 立刻丢
    let mut n = base("P3-critical-drop");
    n.urgency(Urgency::Critical);
    if let Ok(h) = n.show() { drop(h); }
    sleep(Duration::from_millis(1500));

    // P4：forget，永不断开连接
    if let Ok(h) = base("P4-forget").show() { std::mem::forget(h); }
    sleep(Duration::from_millis(1500));

    println!("probe done");
}
