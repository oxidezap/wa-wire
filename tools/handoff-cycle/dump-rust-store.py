"""Dump a whatsapp-rust SQLite store into the rows wa-store-migrate reads.

Bytes go out base64 so the JSON survives; the Node side turns them back.

The one thing worth reading twice is `prekeys.key`. wa-store-migrate's README
says to split it as a 64-byte keypair; it is a libsignal `PreKeyRecordStructure`
protobuf — field 1 the id, field 2 the public key, field 3 the private key. The
engine writes it with `new_pre_key_record(id, &kp).encode_to_vec()`
(`src/prekeys.rs:1376`). Splitting it in halves yields two 32-byte strings that
are neither key.

Device keys are the other way round from how they read: `serialize_keypair`
writes private first, then public (`sqlite_store.rs:847-852`).
"""

import base64
import json
import sqlite3
import sys


def b64(value):
    return None if value is None else base64.b64encode(value).decode()


def split_keypair(blob):
    """Private first, then public — the order the engine writes."""
    if blob is None or len(blob) != 64:
        raise SystemExit(f"a keypair column is {len(blob) if blob else 'null'} bytes, not 64")
    return {"privKey": b64(blob[:32]), "pubKey": b64(blob[32:])}


def read_varint(data, at):
    value = 0
    shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            return value, at


def prekey_record(blob):
    """`PreKeyRecordStructure`: 1 = id, 2 = public key, 3 = private key."""
    fields = {}
    at = 0
    while at < len(blob):
        tag, at = read_varint(blob, at)
        number, wire = tag >> 3, tag & 7
        if wire == 0:
            fields[number], at = read_varint(blob, at)
        elif wire == 2:
            length, at = read_varint(blob, at)
            fields[number] = blob[at : at + length]
            at += length
        else:
            raise SystemExit(f"unexpected wire type {wire} in a prekey record")

    public, private = fields.get(2), fields.get(3)
    if not isinstance(public, bytes) or not isinstance(private, bytes):
        raise SystemExit("a prekey record carries no key pair")
    return fields.get(1), {"pubKey": b64(public), "privKey": b64(private)}


def rows(connection, sql, *args):
    cursor = connection.execute(sql, args)
    names = [column[0] for column in cursor.description]
    return [dict(zip(names, row)) for row in cursor.fetchall()]


def dump(path, device_id=1):
    connection = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    device = rows(connection, "SELECT * FROM device WHERE id = ?", device_id)
    if not device:
        raise SystemExit(f"no device {device_id} in {path}")
    device = device[0]

    snapshot = {
        "device": {
            "registrationId": device["registration_id"],
            "noiseKey": split_keypair(device["noise_key"]),
            "identityKey": split_keypair(device["identity_key"]),
            "signedPreKey": split_keypair(device["signed_pre_key"]),
            "signedPreKeyId": device["signed_pre_key_id"],
            "signedPreKeySignature": b64(device["signed_pre_key_signature"]),
            "advSecretKey": b64(device["adv_secret_key"]),
            "account": b64(device["account"]),
            "pn": device["pn"] or None,
            "lid": device["lid"] or None,
            "pushName": device["push_name"] or None,
            "appVersionPrimary": device["app_version_primary"],
            "appVersionSecondary": device["app_version_secondary"],
            "appVersionTertiary": device["app_version_tertiary"],
            "appVersionLastFetchedMs": device["app_version_last_fetched_ms"],
            "edgeRoutingInfo": b64(device["edge_routing_info"]),
            "propsHash": device["props_hash"] or None,
            "nextPreKeyId": device["next_pre_key_id"],
            "serverHasPrekeys": bool(device["server_has_prekeys"]),
            "nctSalt": b64(device["nct_salt"]),
        },
        "preKeys": [],
        "identities": [],
        "sessions": [],
        "senderKeys": [],
        "senderKeyDevices": [],
        "appStateKeys": [],
        "appStateVersions": [],
        "appStateMutationMacs": [],
        "tcTokens": [],
        "deviceRegistry": [],
        "lidPnMapping": [],
    }

    chain = device["server_cert_chain"]
    if chain:
        snapshot["device"]["serverCertChain"] = {"bytes": b64(chain)}

    for row in rows(connection, "SELECT id, key, uploaded FROM prekeys WHERE device_id = ?", device_id):
        recorded_id, pair = prekey_record(row["key"])
        if recorded_id is not None and recorded_id != row["id"]:
            raise SystemExit(f"prekey {row['id']} records itself as {recorded_id}")
        snapshot["preKeys"].append(
            {"keyId": row["id"], "keyPair": pair, "uploaded": bool(row["uploaded"])}
        )

    for row in rows(connection, "SELECT address, key FROM identities WHERE device_id = ?", device_id):
        snapshot["identities"].append({"address": row["address"], "key": b64(row["key"])})

    for row in rows(connection, "SELECT address, record FROM sessions WHERE device_id = ?", device_id):
        snapshot["sessions"].append({"address": row["address"], "record": b64(row["record"])})

    for row in rows(connection, "SELECT address, record FROM sender_keys WHERE device_id = ?", device_id):
        snapshot["senderKeys"].append({"address": row["address"], "record": b64(row["record"])})

    for row in rows(
        connection,
        "SELECT group_jid, device_jid, has_key FROM sender_key_devices WHERE device_id = ?",
        device_id,
    ):
        snapshot["senderKeyDevices"].append(
            {
                "groupJid": row["group_jid"],
                "deviceJid": row["device_jid"],
                "hasKey": bool(row["has_key"]),
            }
        )

    for row in rows(connection, "SELECT key_id, key_data FROM app_state_keys WHERE device_id = ?", device_id):
        snapshot["appStateKeys"].append({"keyId": b64(row["key_id"]), "keyData": b64(row["key_data"])})

    for row in rows(connection, "SELECT name, state_data FROM app_state_versions WHERE device_id = ?", device_id):
        snapshot["appStateVersions"].append({"name": row["name"], "stateData": b64(row["state_data"])})

    for row in rows(
        connection,
        "SELECT name, version, index_mac, value_mac FROM app_state_mutation_macs WHERE device_id = ?",
        device_id,
    ):
        snapshot["appStateMutationMacs"].append(
            {
                "name": row["name"],
                "version": row["version"],
                "indexMac": b64(row["index_mac"]),
                "valueMac": b64(row["value_mac"]),
            }
        )

    for row in rows(
        connection,
        "SELECT jid, token, token_timestamp, sender_timestamp FROM tc_tokens WHERE device_id = ?",
        device_id,
    ):
        entry = {
            "jid": row["jid"],
            "token": b64(row["token"]),
            "tokenTimestamp": row["token_timestamp"],
        }
        if row["sender_timestamp"] is not None:
            entry["senderTimestamp"] = row["sender_timestamp"]
        snapshot["tcTokens"].append(entry)

    for row in rows(
        connection,
        "SELECT user_id, devices_json, timestamp, phash, raw_id FROM device_registry WHERE device_id = ?",
        device_id,
    ):
        entry = {
            "userJid": row["user_id"],
            "devicesJson": row["devices_json"],
            "timestamp": row["timestamp"],
        }
        if row["phash"]:
            entry["phash"] = row["phash"]
        if row["raw_id"] is not None:
            entry["rawId"] = row["raw_id"]
        snapshot["deviceRegistry"].append(entry)

    for row in rows(
        connection,
        "SELECT lid, phone_number, created_at, learning_source, updated_at FROM lid_pn_mapping WHERE device_id = ?",
        device_id,
    ):
        snapshot["lidPnMapping"].append(
            {
                "lid": row["lid"],
                "phoneNumber": row["phone_number"],
                "createdAt": row["created_at"],
                "learningSource": row["learning_source"],
                "updatedAt": row["updated_at"],
            }
        )

    connection.close()
    return snapshot


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit("usage: dump-rust-store.py <store.db> [device-id]")
    identifier = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    json.dump(dump(sys.argv[1], identifier), sys.stdout)
