use anyhow::{bail, Result};
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerInfo {
    pub uid: u32,
    pub gid: u32,
    pub pid: Option<i32>,
}

/// Verify credentials of a connecting client on the Unix domain socket.
///
/// Under Linux, this uses SO_PEERCRED to verify the peer's UID/GID.
/// Allowed peers:
/// - Root (UID 0)
/// - The UID of the current daemon process
/// - Any user in the `ghostpath`, `wheel`, or `sudo` group
#[cfg(target_os = "linux")]
pub fn verify_peer_credentials(stream: &tokio::net::UnixStream) -> Result<PeerInfo> {
    use nix::sys::socket::getsockopt;
    use nix::sys::socket::sockopt::PeerCredentials;

    let creds = getsockopt(stream, PeerCredentials)
        .map_err(|e| anyhow::anyhow!("Failed to read SO_PEERCRED: {}", e))?;

    let peer = PeerInfo {
        uid: creds.uid(),
        gid: creds.gid(),
        pid: Some(creds.pid()),
    };

    debug!(
        uid = peer.uid,
        gid = peer.gid,
        pid = ?peer.pid,
        "Authenticating peer connection"
    );

    let my_uid = nix::unistd::getuid().as_raw();
    if peer.uid == 0 || peer.uid == my_uid {
        debug!(uid = peer.uid, "Peer authenticated (root or daemon owner)");
        return Ok(peer);
    }

    // Check if peer is member of allowed group (e.g., ghostpath, sudo, wheel)
    if is_user_in_allowed_groups(peer.uid)? {
        debug!(uid = peer.uid, "Peer authenticated via group membership");
        return Ok(peer);
    }

    warn!(
        uid = peer.uid,
        "Rejected unauthorized peer connection via SO_PEERCRED"
    );
    bail!(
        "Access denied: UID {} is not authorized to control GhostPath VPN",
        peer.uid
    );
}

#[cfg(not(target_os = "linux"))]
pub fn verify_peer_credentials(_stream: &tokio::net::UnixStream) -> Result<PeerInfo> {
    debug!("Non-Linux platform: accepting peer connection with mock credentials");
    Ok(PeerInfo {
        uid: 1000,
        gid: 1000,
        pid: None,
    })
}

#[cfg(target_os = "linux")]
fn is_user_in_allowed_groups(uid: u32) -> Result<bool> {
    use nix::unistd::{Group, Uid, User};

    let user = match User::from_uid(Uid::from_raw(uid))? {
        Some(u) => u,
        None => return Ok(false),
    };

    let allowed_group_names = ["ghostpath", "wheel", "sudo", "adm"];
    for name in &allowed_group_names {
        if let Ok(Some(group)) = Group::from_name(name) {
            if group.gid == user.gid || group.mem.contains(&user.name) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_info_creation() {
        let info = PeerInfo {
            uid: 1000,
            gid: 1000,
            pid: Some(1234),
        };
        assert_eq!(info.uid, 1000);
        assert_eq!(info.gid, 1000);
        assert_eq!(info.pid, Some(1234));
    }
}
