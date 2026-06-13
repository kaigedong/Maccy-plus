#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

/// Fixed listen port for reliable reconnection.
constexpr static const uint16_t LISTEN_PORT = 31774;

template<typename T = void>
struct Arc;

struct SyncState;

using MaccySync = Arc<Mutex<SyncState>>;

extern "C" {

MaccySync *maccy_sync_create(const char *device_name, const char *device_id);

void maccy_sync_destroy(MaccySync *sync);

int32_t maccy_sync_start(MaccySync *sync);

int32_t maccy_sync_stop(MaccySync *sync);

void maccy_sync_on_event(MaccySync *sync, void (*cb)(const char *event_json));

int32_t maccy_sync_start_discovery(MaccySync *sync);

int32_t maccy_sync_stop_discovery(MaccySync *sync);

int32_t maccy_sync_add_peer_address(MaccySync *sync, const char *_peer_id, const char *address);

int32_t maccy_sync_request_pairing(MaccySync *sync, const char *peer_id);

int32_t maccy_sync_accept_pairing(MaccySync *sync, const char *peer_id, const char *pin);

int32_t maccy_sync_reject_pairing(MaccySync *sync, const char *peer_id);

int32_t maccy_sync_unpair(MaccySync *sync, const char *peer_id);

int32_t maccy_sync_broadcast_item(MaccySync *sync, const char *item_json);

int32_t maccy_sync_broadcast_deletion(MaccySync *sync, const char *item_id);

int32_t maccy_sync_broadcast_update(MaccySync *sync, const char *item_json);

char *maccy_sync_get_paired_peers(MaccySync *sync);

void maccy_sync_free_string(char *s);

bool maccy_sync_is_running(MaccySync *sync);

char *maccy_sync_get_status(MaccySync *sync);

}  // extern "C"
