use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request as AdblockRequest;
use adblock::Engine;

fn build_engine() -> Engine {
    let mut filter_set = FilterSet::new(true);
    // Örnek filtre listesi (Gerçekte EasyList dosyasından okunur)
    filter_set.add_filters(
        &["-ad-", "/ads/*", "||doubleclick.net^"],
        ParseOptions::default(),
    );
    Engine::from_filter_set(filter_set, true)
}

// Her thread kendi motorunu tutar, böylece Sync gerekmez
thread_local! {
    static ADBLOCK_ENGINE: Engine = build_engine();
}

pub fn should_block(url: &str, resource_type: &str) -> bool {
    let request = match AdblockRequest::new(url, url, resource_type) {
        Ok(r) => r,
        Err(_) => return false,
    };
    ADBLOCK_ENGINE.with(|engine| engine.check_network_request(&request).matched)
}
