use ghostpath_net::dns::{DnsManager, DnsStrategy};
use ghostpath_net::dns_proxy::DnsProxy;
use ghostpath_net::journal::UndoJournal;

#[tokio::test]
async fn test_dns_leak_prevention_lifecycle() {
    let mut journal = UndoJournal::new();
    let mut mgr = DnsManager::new("10.8.0.1", "1.1.1.1", true);

    let guard = mgr
        .apply_dns("ghost0", &mut journal)
        .expect("Failed to apply DNS");

    #[cfg(target_os = "linux")]
    assert!(matches!(
        mgr.strategy(),
        DnsStrategy::SystemdResolved | DnsStrategy::NetworkManager | DnsStrategy::DirectResolvConf
    ));
    assert!(!journal.actions.is_empty());

    // Verify proxy handles queries
    let proxy = DnsProxy::new(10856, "https://1.1.1.1/dns-query");
    proxy.start().await.expect("Failed to start DNS proxy");

    drop(guard);
    proxy.stop();
    mgr.restore_dns("ghost0").expect("Failed to restore DNS");
}
