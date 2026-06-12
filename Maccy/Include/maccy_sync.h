#ifndef MACCY_SYNC_H
#define MACCY_SYNC_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MaccySync MaccySync;

typedef void (*MaccyPeerDiscoveredCallback)(const char* peer_id, const char* display_name, const char* addresses);
typedef void (*MaccyPeerLostCallback)(const char* peer_id);
typedef void (*MaccyPairingRequestCallback)(const char* peer_id, const char* display_name, const char* pin);
typedef void (*MaccyPairingCompleteCallback)(const char* peer_id, bool success);
typedef void (*MaccyItemReceivedCallback)(const char* item_json);
typedef void (*MaccyItemDeletedCallback)(const char* item_id);
typedef void (*MaccyItemUpdatedCallback)(const char* item_json);
typedef void (*MaccyErrorCallback)(int32_t code, const char* message);

#define MACCY_SYNC_OK              0
#define MACCY_SYNC_ERR_INIT        1
#define MACCY_SYNC_ERR_PAIRING     2
#define MACCY_SYNC_ERR_NETWORK     3
#define MACCY_SYNC_ERR_INVALID_ARG 4
#define MACCY_SYNC_ERR_NOT_RUNNING 5

MaccySync* maccy_sync_create(const char* device_name, const char* device_id);
void maccy_sync_destroy(MaccySync* sync);

int32_t maccy_sync_start(MaccySync* sync);
int32_t maccy_sync_stop(MaccySync* sync);

void maccy_sync_on_peer_discovered(MaccySync* sync, MaccyPeerDiscoveredCallback cb);
void maccy_sync_on_peer_lost(MaccySync* sync, MaccyPeerLostCallback cb);
void maccy_sync_on_pairing_request(MaccySync* sync, MaccyPairingRequestCallback cb);
void maccy_sync_on_pairing_complete(MaccySync* sync, MaccyPairingCompleteCallback cb);
void maccy_sync_on_sync_item_received(MaccySync* sync, MaccyItemReceivedCallback cb);
void maccy_sync_on_sync_item_deleted(MaccySync* sync, MaccyItemDeletedCallback cb);
void maccy_sync_on_sync_item_updated(MaccySync* sync, MaccyItemUpdatedCallback cb);
void maccy_sync_on_error(MaccySync* sync, MaccyErrorCallback cb);

int32_t maccy_sync_start_discovery(MaccySync* sync);
int32_t maccy_sync_stop_discovery(MaccySync* sync);

int32_t maccy_sync_request_pairing(MaccySync* sync, const char* peer_id);
int32_t maccy_sync_accept_pairing(MaccySync* sync, const char* peer_id, const char* pin);
int32_t maccy_sync_reject_pairing(MaccySync* sync, const char* peer_id);

int32_t maccy_sync_broadcast_item(MaccySync* sync, const char* item_json);
int32_t maccy_sync_broadcast_deletion(MaccySync* sync, const char* item_id);
int32_t maccy_sync_broadcast_update(MaccySync* sync, const char* item_json);

int32_t maccy_sync_add_peer_address(MaccySync* sync, const char* peer_id, const char* address);

char* maccy_sync_get_paired_peers(MaccySync* sync);
void maccy_sync_free_string(char* s);
int32_t maccy_sync_unpair(MaccySync* sync, const char* peer_id);

bool maccy_sync_is_running(MaccySync* sync);
char* maccy_sync_get_status(MaccySync* sync);

#ifdef __cplusplus
}
#endif

#endif
