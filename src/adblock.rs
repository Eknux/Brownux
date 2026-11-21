use adblock::engine::Engine;
use adblock::lists::FilterSet;
use adblock::request::Request as AdblockRequest;
use once_cell::sync::Lazy;

static ADBLOCK_ENGINE: Lazy<Engine> = Lazy::new(|| {
    let mut filter_set = FilterSet::new(true);
    // Örnek filtre listesi (Gerçekte EasyList dosyasından okunur)
    filter_set.add_filters(&[
        "-ad-",
        "/ads/*",
        "||doubleclick.net^",
    ], true);
    Engine::from_filter_set(filter_set, true)
});

pub fn should_block(url: &str, resource_type: &str) -> bool {
    let request = AdblockRequest::new(url, resource_type, "")
        .expect("Geçersiz adblock isteği");
    ADBLOCK_ENGINE.check_network_request(&request).matched
}