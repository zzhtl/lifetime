// Scheduler 内/外消息定义

use crate::reminders::ReminderKind;
use std::time::Duration;

/// UI → Scheduler 控制指令
#[allow(dead_code)] // Snooze 留作未来"暂缓 5/10 min"按钮使用
#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Pause,
    Resume,
    Stop,
    /// 跳过当前一个提醒
    Skip(ReminderKind),
    /// 暂缓一段时间
    Snooze(ReminderKind, Duration),
    /// 用户确认完成（点击休息窗的"完成"）
    AcknowledgeBreak(ReminderKind),
    /// 强制立即触发某种提醒（调试/手动）
    TriggerNow(ReminderKind),
    /// 退出线程
    Quit,
}

/// Scheduler → UI 事件
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    /// 到点触发某种提醒
    Triggered(ReminderKind),
    /// 状态机变化（开始/暂停/恢复/停止）
    StateChanged(RunState),
    /// 心跳：当前会话已运行秒数
    Heartbeat { running_secs: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Idle,
    Running,
    Paused,
}
