// 呼吸引导引擎：呼吸法预设 + 相位状态机（纯逻辑，可单测）
//
// 与「养生修炼」里的功法文本（practices）不同，这里是驱动可视化节拍器的
// 精确节律定义：每种呼吸法 = 一组相位（吸/屏/呼/屏）配时长；引擎按已经过
// 的秒数「反算」当前处于第几轮、哪个相位、相位内进度，从而驱动圆圈缩放动画。
// 用 Instant 反算而非逐帧累加，即便某些帧被跳过（窗口失焦）回来也能自动校准，零漂移。

use std::time::Instant;

/// 呼吸相位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseKind {
    /// 吸气：圆圈由小变大
    Inhale,
    /// 屏息（满）：保持最大
    HoldIn,
    /// 呼气：由大变小
    Exhale,
    /// 屏息（空）：保持最小
    HoldOut,
}

impl PhaseKind {
    pub fn label(self) -> &'static str {
        match self {
            PhaseKind::Inhale => "吸气",
            PhaseKind::HoldIn => "屏住",
            PhaseKind::Exhale => "呼气",
            PhaseKind::HoldOut => "屏住",
        }
    }
}

/// 一个相位：类型 + 时长（秒）
#[derive(Debug, Clone, Copy)]
pub struct Phase {
    pub kind: PhaseKind,
    pub secs: f32,
}

const fn p(kind: PhaseKind, secs: f32) -> Phase {
    Phase { kind, secs }
}

/// 一种呼吸法预设
pub struct BreathingPattern {
    /// 稳定 key（入库 / 关键词匹配用）
    pub key: &'static str,
    pub name: &'static str,
    /// 目标分组：助眠放松 / 快速减压 / 专注稳定 / 日常平衡
    pub goal: &'static str,
    /// 一句话说明
    pub tagline: &'static str,
    /// 一轮的相位序列
    pub phases: &'static [Phase],
    /// 默认目标轮数
    pub default_cycles: u32,
    /// 主题色 RGB
    pub accent: (u8, u8, u8),
    /// 中性现代解释（无绝对疗效词）
    pub note: &'static str,
    pub source_name: &'static str,
    pub source_url: &'static str,
}

impl BreathingPattern {
    /// 一轮总时长（秒）
    pub fn cycle_secs(&self) -> f32 {
        self.phases.iter().map(|ph| ph.secs).sum()
    }
}

/// 全部呼吸法预设。顺序即练习台选择器的展示顺序。
pub static BREATHING_PATTERNS: &[BreathingPattern] = &[
    BreathingPattern {
        key: "coherent",
        name: "共振呼吸",
        goal: "日常平衡",
        tagline: "吸呼各 5.5 秒，每分钟约 6 息",
        phases: &[p(PhaseKind::Inhale, 5.5), p(PhaseKind::Exhale, 5.5)],
        default_cycles: 10,
        accent: (0x6f, 0xc2, 0xb8),
        note: "每分钟约 6 次、吸呼等长的慢呼吸，常见于放松与心率变异性相关练习。",
        source_name: "Harvard Health: Breath control helps quell stress",
        source_url: "https://www.health.harvard.edu/mind-and-mood/relaxation-techniques-breath-control-helps-quell-errant-stress-response",
    },
    BreathingPattern {
        key: "478",
        name: "4-7-8 呼吸",
        goal: "助眠放松",
        tagline: "吸 4 · 屏 7 · 呼 8，延长呼气",
        phases: &[
            p(PhaseKind::Inhale, 4.0),
            p(PhaseKind::HoldIn, 7.0),
            p(PhaseKind::Exhale, 8.0),
        ],
        default_cycles: 4,
        accent: (0x8f, 0x9f, 0xe0),
        note: "吸 4、屏 7、呼 8 的固定节律，用较长的呼气帮助放松，常用于睡前。",
        source_name: "Cleveland Clinic: 4-7-8 Breathing",
        source_url: "https://health.clevelandclinic.org/4-7-8-breathing",
    },
    BreathingPattern {
        key: "box",
        name: "箱式呼吸",
        goal: "专注稳定",
        tagline: "吸 4 · 屏 4 · 呼 4 · 屏 4，四方等长",
        phases: &[
            p(PhaseKind::Inhale, 4.0),
            p(PhaseKind::HoldIn, 4.0),
            p(PhaseKind::Exhale, 4.0),
            p(PhaseKind::HoldOut, 4.0),
        ],
        default_cycles: 6,
        accent: (0x68, 0xb8, 0xd8),
        note: "吸—屏—呼—屏 各 4 拍的等长节律，有助于收摄注意力与稳定情绪。",
        source_name: "Cleveland Clinic: Box Breathing",
        source_url: "https://health.clevelandclinic.org/box-breathing-benefits",
    },
    BreathingPattern {
        key: "sigh",
        name: "生理叹息",
        goal: "快速减压",
        tagline: "双吸一口，缓缓长呼",
        phases: &[p(PhaseKind::Inhale, 3.0), p(PhaseKind::Exhale, 7.0)],
        default_cycles: 5,
        accent: (0xe0, 0x8f, 0x77),
        note: "两段吸气后缓慢长呼（生理叹息），是一种快速平复紧张的呼吸方式。",
        source_name: "NIH NCCIH Relaxation Techniques",
        source_url: "https://www.nccih.nih.gov/health/relaxation-techniques-what-you-need-to-know",
    },
    BreathingPattern {
        key: "belly",
        name: "腹式深呼吸",
        goal: "日常平衡",
        tagline: "吸 4 · 呼 6，膈肌带动",
        phases: &[p(PhaseKind::Inhale, 4.0), p(PhaseKind::Exhale, 6.0)],
        default_cycles: 8,
        accent: (0x79, 0xc2, 0x8a),
        note: "用横膈膜带动的缓慢腹式呼吸，帮助放松、减少浅快的胸式呼吸。",
        source_name: "Harvard Health: Learning diaphragmatic breathing",
        source_url: "https://www.health.harvard.edu/healthbeat/learning-diaphragmatic-breathing",
    },
    BreathingPattern {
        key: "exhale",
        name: "延长呼气",
        goal: "快速减压",
        tagline: "吸 4 · 呼 8，呼倍于吸",
        phases: &[p(PhaseKind::Inhale, 4.0), p(PhaseKind::Exhale, 8.0)],
        default_cycles: 6,
        accent: (0xd0, 0xa5, 0x5f),
        note: "让呼气长于吸气，偏向放松的一侧，重点在平稳缓慢而非用力。",
        source_name: "NIH NCCIH Relaxation Techniques",
        source_url: "https://www.nccih.nih.gov/health/relaxation-techniques-what-you-need-to-know",
    },
];

/// 某一时刻的相位快照（纯函数产物，供 UI 画圈与显示）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseAt {
    /// 已完成的整轮数（从 0 开始）
    pub cycle: u32,
    /// 当前相位在 phases 中的下标
    pub phase_index: usize,
    pub kind: PhaseKind,
    /// 当前相位内进度 0..1
    pub phase_progress: f32,
    /// 当前相位剩余秒数
    pub phase_remaining: f32,
    /// 圆圈半径插值 0..1（已缓动：0=最小，1=最大）
    pub radius_t: f32,
}

/// smoothstep 缓动，让吸/呼像真实呼吸而非匀速直线。
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// 按已经过秒数反算当前相位（纯函数，零漂移）。
pub fn phase_at(pattern: &BreathingPattern, elapsed: f32) -> PhaseAt {
    let cycle_secs = pattern.cycle_secs().max(0.001);
    let elapsed = elapsed.max(0.0);
    let cycle = (elapsed / cycle_secs).floor() as u32;
    let mut within = elapsed - cycle as f32 * cycle_secs; // 0..cycle_secs
    let last = pattern.phases.len().saturating_sub(1);
    for (i, ph) in pattern.phases.iter().enumerate() {
        if within < ph.secs || i == last {
            let dur = ph.secs.max(0.001);
            let prog = (within / dur).clamp(0.0, 1.0);
            let radius_t = match ph.kind {
                PhaseKind::Inhale => smoothstep(prog),
                PhaseKind::HoldIn => 1.0,
                PhaseKind::Exhale => 1.0 - smoothstep(prog),
                PhaseKind::HoldOut => 0.0,
            };
            return PhaseAt {
                cycle,
                phase_index: i,
                kind: ph.kind,
                phase_progress: prog,
                phase_remaining: (ph.secs - within).max(0.0),
                radius_t,
            };
        }
        within -= ph.secs;
    }
    // 理论不可达（至少一个相位时上面必返回）
    PhaseAt {
        cycle,
        phase_index: 0,
        kind: PhaseKind::Inhale,
        phase_progress: 0.0,
        phase_remaining: 0.0,
        radius_t: 0.0,
    }
}

/// 练习台运行时状态（挂在 App 上）
pub struct BreathingState {
    pub pattern_idx: usize,
    pub target_cycles: u32,
    pub running: bool,
    pub sound_on: bool,
    /// 本次连续运行的起点（None 表示未在计时）
    started_at: Option<Instant>,
    /// 暂停前已累计的秒数
    accumulated: f32,
    /// 上一帧所处 (cycle, phase_index)，用于检测相位切换以触发提示音
    pub last_boundary: Option<(u32, usize)>,
    /// 本次是否已记账，避免重复落库
    pub session_logged: bool,
}

impl Default for BreathingState {
    fn default() -> Self {
        Self {
            pattern_idx: 0,
            target_cycles: BREATHING_PATTERNS[0].default_cycles,
            running: false,
            sound_on: false,
            started_at: None,
            accumulated: 0.0,
            last_boundary: None,
            session_logged: false,
        }
    }
}

impl BreathingState {
    /// 当前预设（'static，不借用 self，故不影响后续对 App 的可变借用）
    pub fn pattern(&self) -> &'static BreathingPattern {
        &BREATHING_PATTERNS[self.pattern_idx.min(BREATHING_PATTERNS.len() - 1)]
    }

    /// 已经过秒数（运行中含实时增量）
    pub fn elapsed(&self) -> f32 {
        self.accumulated
            + self
                .started_at
                .map(|t| t.elapsed().as_secs_f32())
                .unwrap_or(0.0)
    }

    /// 切换预设（会重置进度并采用该预设的默认轮数）
    pub fn select(&mut self, idx: usize) {
        let idx = idx.min(BREATHING_PATTERNS.len() - 1);
        if idx == self.pattern_idx && self.elapsed() == 0.0 {
            return;
        }
        self.reset();
        self.pattern_idx = idx;
        self.target_cycles = self.pattern().default_cycles;
    }

    /// 开始 / 继续（若上次已完成则重新来过）
    pub fn start(&mut self) {
        if self.running {
            return;
        }
        if self.session_logged || self.completed_cycles() >= self.target_cycles {
            self.reset();
        }
        self.started_at = Some(Instant::now());
        self.running = true;
    }

    /// 暂停（把已计时间折进 accumulated，冻结进度）
    pub fn pause(&mut self) {
        if !self.running {
            return;
        }
        self.accumulated = self.elapsed();
        self.started_at = None;
        self.running = false;
    }

    /// 达标收尾：冻结进度并标记已记账
    pub fn mark_finished(&mut self) {
        self.accumulated = self.elapsed();
        self.started_at = None;
        self.running = false;
        self.session_logged = true;
    }

    /// 重置到起始状态
    pub fn reset(&mut self) {
        self.running = false;
        self.started_at = None;
        self.accumulated = 0.0;
        self.last_boundary = None;
        self.session_logged = false;
    }

    /// 当前相位快照
    pub fn current(&self) -> PhaseAt {
        phase_at(self.pattern(), self.elapsed())
    }

    /// 已完成的整轮数
    pub fn completed_cycles(&self) -> u32 {
        let cs = self.pattern().cycle_secs().max(0.001);
        (self.elapsed() / cs).floor() as u32
    }

    #[cfg(test)]
    fn force_elapsed(&mut self, secs: f32) {
        self.accumulated = secs;
        self.started_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(key: &str) -> &'static BreathingPattern {
        BREATHING_PATTERNS.iter().find(|p| p.key == key).unwrap()
    }

    #[test]
    fn patterns_are_valid() {
        assert!(!BREATHING_PATTERNS.is_empty());
        let banned = ["包治", "根治", "保证长生", "永生", "百病不生", "药到病除"];
        let mut keys = std::collections::HashSet::new();
        for pat in BREATHING_PATTERNS {
            assert!(keys.insert(pat.key), "预设 key 重复: {}", pat.key);
            assert!(!pat.phases.is_empty(), "{} 无相位", pat.name);
            assert!(pat.cycle_secs() > 0.0, "{} 单轮时长为 0", pat.name);
            for ph in pat.phases {
                assert!(ph.secs > 0.0, "{} 存在非正时长相位", pat.name);
            }
            assert!(pat.default_cycles >= 1, "{} 默认轮数应 >=1", pat.name);
            assert!(
                pat.source_url.starts_with("https://"),
                "{} 来源非 https",
                pat.name
            );
            assert!(!pat.source_name.trim().is_empty(), "{} 来源名为空", pat.name);
            let text = format!("{}{}{}", pat.name, pat.tagline, pat.note);
            for w in banned {
                assert!(!text.contains(w), "{} 含绝对疗效词 {w}", pat.name);
            }
        }
    }

    #[test]
    fn phase_at_478_boundaries() {
        let p = pattern("478"); // 吸4 屏7 呼8，一轮 19s
        assert_eq!(p.cycle_secs(), 19.0);

        let a = phase_at(p, 0.0);
        assert_eq!(a.kind, PhaseKind::Inhale);
        assert_eq!(a.cycle, 0);
        assert!(a.radius_t.abs() < 1e-4);

        assert_eq!(phase_at(p, 2.0).kind, PhaseKind::Inhale);
        assert_eq!(phase_at(p, 4.0).kind, PhaseKind::HoldIn);
        assert_eq!(phase_at(p, 10.9).kind, PhaseKind::HoldIn);
        assert_eq!(phase_at(p, 11.0).kind, PhaseKind::Exhale);
        assert_eq!(phase_at(p, 18.9).kind, PhaseKind::Exhale);

        // 进入第二轮
        let b = phase_at(p, 19.0);
        assert_eq!(b.cycle, 1);
        assert_eq!(b.kind, PhaseKind::Inhale);
    }

    #[test]
    fn radius_curve_matches_phase() {
        let p = pattern("478");
        // 屏息（满）半径恒为 1
        assert!((phase_at(p, 7.0).radius_t - 1.0).abs() < 1e-4);
        // 吸气临近结束半径接近 1
        assert!(phase_at(p, 3.9).radius_t > 0.9);
        // 呼气中段半径在 (0,1) 且下降
        let mid = phase_at(p, 15.0).radius_t;
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn box_has_four_symmetric_phases() {
        let p = pattern("box");
        assert_eq!(p.cycle_secs(), 16.0);
        assert_eq!(phase_at(p, 0.0).kind, PhaseKind::Inhale);
        assert_eq!(phase_at(p, 4.0).kind, PhaseKind::HoldIn);
        assert_eq!(phase_at(p, 8.0).kind, PhaseKind::Exhale);
        assert_eq!(phase_at(p, 12.0).kind, PhaseKind::HoldOut);
        // 屏空半径为 0
        assert!(phase_at(p, 12.5).radius_t.abs() < 1e-4);
    }

    #[test]
    fn state_completed_cycles_and_reset() {
        let mut s = BreathingState::default();
        let idx = BREATHING_PATTERNS.iter().position(|p| p.key == "478").unwrap();
        s.select(idx);
        s.target_cycles = 3;
        s.force_elapsed(19.0 * 2.0 + 5.0); // 2 整轮 + 第三轮进行中
        assert_eq!(s.completed_cycles(), 2);
        assert_eq!(s.current().cycle, 2);
        s.reset();
        assert_eq!(s.elapsed(), 0.0);
        assert_eq!(s.completed_cycles(), 0);
    }
}
