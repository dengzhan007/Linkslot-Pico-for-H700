mod common;

use std::time::{Duration, Instant};

use slot::link_art_builder::LinkArtBuilder;
use slot::link_screen::{LinkSprites, Sprite};
use slot_ui::{Language, TexId, ADAPTER_W, PORT_W, ROUTER_W};

#[test]
fn the_art_arrives_once_and_only_once() {
    let builder = LinkArtBuilder::spawn(Language::ZhCn);
    let deadline = Instant::now() + Duration::from_secs(60);
    let art = loop {
        if let Some(art) = builder.take() {
            break art;
        }
        assert!(Instant::now() < deadline, "the link art never arrived");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(art.port.w, PORT_W);
    assert_eq!(art.adapter.w, ADAPTER_W);
    assert_eq!(art.router.w, ROUTER_W);
    assert!(builder.take().is_none(), "the art was built twice");
}

#[test]
fn an_app_with_sprites_is_ready_to_draw_the_art() {
    let d = common::tmp_root_with_carts(&["Zzz"]);
    let mut app = common::boot(d.path());
    assert!(!app.link_sprites_ready());
    let s = |n| Sprite {
        tex: TexId::from_raw(n),
        w: 1,
        h: 1,
    };
    app.set_link_sprites(LinkSprites {
        port: s(1),
        plug_host: s(2),
        plug_join: s(3),
        adapter: s(4),
        router: s(16),
        hotspot_host: s(17),
        hotspot_join: s(18),
        wifi_host: s(19),
        wifi_join: s(20),
        wifi_line_host: s(21),
        wifi_line_join: s(22),
        hotspot_arcs_cyan: [s(23), s(24), s(25)],
        hotspot_arcs_amber: [s(26), s(27), s(28)],
        hotspot_arcs_amber_solid: [s(34), s(35), s(36)],
        ring_cyan: s(29),
        ring_amber: s(30),
        dot_cyan: s(31),
        dot_amber: s(32),
        check: s(33),
        arcs_right: [s(7), s(8), s(9)],
        arcs_left: [s(10), s(11), s(12)],
        clicks: s(13),
        arrow_left: s(14),
        arrow_right: s(15),
    });
    assert!(app.link_sprites_ready());
}
