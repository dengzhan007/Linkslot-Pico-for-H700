//! Phase 5a「插板 / 投币 / attract」逻辑层的**独立验收**（QA）。
//!
//! 只放测试代码，由 `lib.rs` 的 `#[cfg(test)] mod arcade_stage_qa;` 挂进来。
//!
//! 契约（lead 定稿）与本文件的对应：
//! - 时间模型是 **dt 累加**：`tick(dt)` 内部 `dt.clamp(0.0, MAX_TICK)`；
//! - 插入 0.180s；投币限流 0.240s；`credits` 上限 9；FREE PLAY 默认开；
//! - **第 10 枚：`credits == 9` 时 `coin()` 返回 `false` 且不改任何状态**（不重置 `coin_t`）；
//! - **限流窗口内（`coin_t > 0`）`coin()` 返回 `false` 且不改状态**；
//! - 门控：`Inserting` 期间所有输入无效（`accepts_input()` 与 `is_busy()` 必须互反）；
//! - `back()` 只在 `Attract` 成立；`insert(boards)` 只在 `Main` 且 `boards > 0` 成立。
//!
//! **边界不咬浮点等号**：f32 下 `0.179 + 0.001` 可能落在 `0.17999999`，所以插板用
//! `0.179` + `0.002`、限流用 `0.239` + `0.002`，两侧各留 1ms 余量；生产代码不加 epsilon。
//!
//! 另：`tick` 会把单帧夹到 [`MAX_TICK`]，所以推进指定秒数必须分多帧（见 [`tick_for`]）。

use slot_store::Platform;

use crate::arcade::{
    ArcadeAction, ArcadeKey, ArcadeScreen, ArcadeStage, COIN_SECS, INSERT_SECS, MAX_CREDITS,
    MAX_TICK,
};

/// 有板时传给 `insert` 的数量（`> 0` 即允许插板）。
const BOARDS: usize = 2;

fn new_screen() -> ArcadeScreen {
    ArcadeScreen::new(Platform::Arcade)
}

/// 推进指定秒数；`tick` 会把单帧夹到 [`MAX_TICK`]，所以必须分多帧喂。
fn tick_for(screen: &mut ArcadeScreen, secs: f32) {
    let mut left = secs;
    while left > 0.0 {
        let step = left.min(MAX_TICK);
        screen.tick(step);
        left -= step;
    }
}

/// 走完一次插板（有余量的两发），停在 `Attract`。
fn to_attract(screen: &mut ArcadeScreen) {
    assert!(screen.insert(BOARDS), "有板时 insert(2) 应返回 true");
    tick_for(screen, INSERT_SECS - 0.001);
    screen.tick(0.002);
    assert_eq!(screen.stage(), ArcadeStage::Attract, "插板后应进入 attract");
}

// ---------------------------------------------------------------- ① 插板：闸门 + 计时

/// 插入未满 `INSERT_SECS` 时仍在 `Inserting`，且这段时间所有输入被闸门挡住。
#[test]
fn the_insert_animation_gates_input_until_it_finishes() {
    let mut screen = new_screen();
    assert_eq!(screen.stage(), ArcadeStage::Main);
    assert!(screen.accepts_input(), "Main 下应当接受输入");
    assert!(!screen.is_busy(), "Main 下不该是 busy");

    assert!(screen.insert(BOARDS));
    assert_eq!(screen.stage(), ArcadeStage::Inserting);
    assert!(!screen.accepts_input(), "插入期必须拒绝输入");
    assert!(screen.is_busy(), "插入期必须 busy");

    tick_for(&mut screen, INSERT_SECS - 0.001); // 0.179
    assert_eq!(
        screen.stage(),
        ArcadeStage::Inserting,
        "0.179s 就完成了插板（动画被跳过）"
    );
    assert!(!screen.accepts_input(), "插入未完成时仍应拒绝输入");

    screen.tick(0.002); // 累计 0.181 > 0.180
    assert_eq!(screen.stage(), ArcadeStage::Attract);
    assert!(screen.accepts_input(), "Attract 下应当接受输入");
    assert!(!screen.is_busy(), "Attract 下不该是 busy");
}

/// `dt` 必须被钳到 `MAX_TICK`：一帧掉到 1s 也不能把插板动画一帧跳过。
#[test]
fn a_huge_frame_cannot_skip_the_insert_animation() {
    let mut screen = new_screen();
    assert!(screen.insert(BOARDS));

    // 1.0s 的三帧：若正确钳到 0.05，累计只推进 0.15s，仍在 Inserting。
    for frame in 0..3 {
        screen.tick(1.0);
        assert_eq!(
            screen.stage(),
            ArcadeStage::Inserting,
            "第 {} 帧后插板就完成了（dt 没有被钳到 MAX_TICK）",
            frame + 1
        );
    }
    screen.tick(1.0); // 累计 0.20 ≥ 0.18
    assert_eq!(screen.stage(), ArcadeStage::Attract);
}

/// 插板期间货架相关的操作必须完全无效（不能"插着板还在翻板/换筛选"）。
#[test]
fn shelf_actions_are_ignored_while_inserting() {
    let mut screen = new_screen();
    let systems = vec!["CPS-1".to_string(), "CPS-2".to_string()];
    screen.index = 1;
    let filter_before = screen.filter.clone();

    assert!(screen.insert(BOARDS));
    screen.select_next(3);
    assert_eq!(screen.index, 1, "插入期 select_next 不该改选中项");
    screen.select_prev(3);
    assert_eq!(screen.index, 1, "插入期 select_prev 不该改选中项");
    screen.cycle_filter(&systems);
    assert_eq!(screen.filter, filter_before, "插入期不该换筛选");

    // Attract 下：`key()` 整体 no-op（键由 app 直接分派 coin/start/back），而且
    // 货架操作也一律 no-op —— 门从 `is_busy()` 收紧到"仅 Main"，机台已在跑还翻板
    // 没有意义。这两条一起钉住"Attract 不是可导航状态"。
    tick_for(&mut screen, INSERT_SECS + 0.001);
    assert_eq!(screen.stage(), ArcadeStage::Attract);
    let before = screen.index;
    assert_eq!(
        screen.key(ArcadeKey::Right, 3, &systems),
        ArcadeAction::None,
        "Attract 下 key() 应当 no-op"
    );
    assert_eq!(screen.index, before, "Attract 下 key() 不该改选中项");
    screen.select_next(3);
    assert_eq!(screen.index, before, "Attract 下 select_next 不该改选中项");
    screen.select_prev(3);
    assert_eq!(screen.index, before, "Attract 下 select_prev 不该改选中项");
    screen.cycle_filter(&systems);
    assert_eq!(
        screen.filter, filter_before,
        "Attract 下 cycle_filter 不该换筛选"
    );
}

// ---------------------------------------------------------------- ② 投币：上限与限流

/// 上限 `MAX_CREDITS`：第 `MAX_CREDITS + 1` 枚必须被拒绝，且**不改任何状态**。
#[test]
fn the_tenth_coin_is_refused() {
    let mut screen = new_screen();
    to_attract(&mut screen);

    for n in 1..=MAX_CREDITS {
        assert!(screen.coin(), "第 {n} 枚应当被接受");
        assert_eq!(screen.credits(), n, "第 {n} 枚后 credits 应为 {n}");
        tick_for(&mut screen, COIN_SECS + 0.001); // 放掉限流窗口
    }
    assert_eq!(screen.credits(), MAX_CREDITS);

    assert!(
        !screen.coin(),
        "第 {} 枚应当被拒绝（credits 已满 {MAX_CREDITS}）",
        MAX_CREDITS + 1
    );
    assert_eq!(
        screen.credits(),
        MAX_CREDITS,
        "被拒的投币不该改变 credits"
    );
    // 拒绝不该重置限流窗口：窗口已过，立刻再投仍然是 false（因为满额）。
    assert!(!screen.coin(), "满额被拒后立刻再投仍应被拒");
    assert_eq!(screen.credits(), MAX_CREDITS);
}

/// 限流 `COIN_SECS`：窗口内的第二枚必须被拒绝**且不改状态**，窗口过后才接受。
#[test]
fn the_coin_rate_limit_opens_after_the_window() {
    let mut screen = new_screen();
    to_attract(&mut screen);

    assert!(screen.coin(), "第一枚应当被接受");
    assert_eq!(screen.credits(), 1);

    tick_for(&mut screen, COIN_SECS - 0.001); // 0.239，窗口还差 1ms
    assert!(
        !screen.coin(),
        "0.239s 内又接受了一枚（0.240s 的投币限流没有生效）"
    );
    assert_eq!(screen.credits(), 1, "被限流拒绝的投币不该改变 credits");

    tick_for(&mut screen, 0.002); // 累计 0.241 ≥ 0.240
    assert!(screen.coin(), "限流窗口过后仍不可投");
    assert_eq!(screen.credits(), 2, "两枚都应计入 credits");
}

// ---------------------------------------------------------------- ③ FREE PLAY 与开始

/// FREE PLAY 打开时：投币仍计数、`start()` 不扣；关掉后：`start()` 扣 1，0 枚时不能开始。
#[test]
fn free_play_counts_coins_but_never_spends_them() {
    let mut screen = new_screen();
    to_attract(&mut screen);
    assert!(screen.free_play(), "FREE PLAY 默认应当打开");
    assert_eq!(screen.credits(), 0);
    assert!(screen.start(), "FREE PLAY 下 0 枚币也应当能开始");

    assert!(screen.coin());
    assert_eq!(screen.credits(), 1);
    assert!(screen.start(), "FREE PLAY 下应当可以开始");
    assert_eq!(screen.credits(), 1, "FREE PLAY 下 start() 不应当扣币");

    // 关掉 FREE PLAY：一枚币换一次开始，用完就不能再开始。
    screen.set_free_play(false);
    assert!(!screen.free_play());
    assert!(screen.start(), "非 FREE PLAY 且有 1 枚时应能开始");
    assert_eq!(screen.credits(), 0, "非 FREE PLAY 下 start() 应当扣到 0");
    assert!(!screen.start(), "0 枚币时不应能开始");
    assert_eq!(screen.credits(), 0, "0 枚时 start() 失败也不该变成负数");
}

// ---------------------------------------------------------------- ④ back / insert 的守卫

/// `back()` 只把 `Attract` 拉回 `Main` 并清零 credits；`Main`/`Inserting` 下返回 `false`。
#[test]
fn back_only_returns_from_attract_and_clears_the_credits() {
    let mut screen = new_screen();
    assert!(!screen.back(), "Main 下 back() 应当返回 false");
    assert_eq!(screen.stage(), ArcadeStage::Main, "Main 下 back() 不该改状态");

    assert!(screen.insert(BOARDS));
    assert!(!screen.back(), "Inserting 下 back() 应当返回 false");
    assert_eq!(
        screen.stage(),
        ArcadeStage::Inserting,
        "Inserting 下 back() 不该改状态"
    );

    tick_for(&mut screen, INSERT_SECS + 0.001);
    assert_eq!(screen.stage(), ArcadeStage::Attract);
    assert!(screen.coin());
    assert_eq!(screen.credits(), 1);

    assert!(screen.back(), "Attract 下 back() 应当返回 true");
    assert_eq!(screen.stage(), ArcadeStage::Main, "back() 后应回 Main");
    assert_eq!(screen.credits(), 0, "back() 回 Main 后 credits 没有清零");
}

/// `insert(boards)` 只在 `Main` 且 `boards > 0` 时成立；其余情况 `false` 且不改状态。
#[test]
fn insert_is_only_allowed_from_main_with_boards() {
    let mut screen = new_screen();
    assert!(screen.insert(BOARDS));
    assert!(!screen.insert(BOARDS), "Inserting 下不能再插一次");
    assert_eq!(
        screen.stage(),
        ArcadeStage::Inserting,
        "被拒的 insert 不该改状态"
    );

    tick_for(&mut screen, INSERT_SECS + 0.001);
    assert_eq!(screen.stage(), ArcadeStage::Attract);
    assert!(!screen.insert(BOARDS), "Attract 下不该重新触插板");
    assert_eq!(
        screen.stage(),
        ArcadeStage::Attract,
        "被拒的 insert 不该改状态"
    );

    // 空板：`insert(0)` 必须是 false，且停在 Main。
    let mut empty = new_screen();
    assert!(!empty.insert(0), "没有板子时不该允许插板");
    assert_eq!(
        empty.stage(),
        ArcadeStage::Main,
        "被拒的 insert(0) 不该改状态"
    );
    assert!(empty.insert(BOARDS), "同一屏在有板时仍应能插板");
}

// ---------------------------------------------------------------- ⑤ 闸门（两套必须互反）

/// `accepts_input()` 与 `is_busy()` 必须是同一判据的反向，且只有 `Inserting` 挡输入。
///
/// 如果实现里留了两套判据（`app` 用 `is_busy()`、我断言 `accepts_input()`），这条会红——
/// 否则"闸门"这条验收测的就不是生产实际用的那个。
#[test]
fn accepts_input_and_busy_are_exact_inverses() {
    let mut screen = new_screen();
    for stage in 0..3 {
        match stage {
            0 => assert_eq!(screen.stage(), ArcadeStage::Main),
            1 => assert!(screen.insert(BOARDS)),
            _ => tick_for(&mut screen, INSERT_SECS + 0.001),
        }
        assert_eq!(
            screen.accepts_input(),
            !screen.is_busy(),
            "stage={:?} 时 accepts_input() 与 !is_busy() 不一致（两套闸门分叉）",
            screen.stage()
        );
        assert_eq!(
            screen.is_busy(),
            screen.stage() == ArcadeStage::Inserting,
            "stage={:?} 时 is_busy() 与阶段不符",
            screen.stage()
        );
    }
}
