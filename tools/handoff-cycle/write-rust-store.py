"""Write a migrated session back into a whatsapp-rust SQLite store.

The return leg. It takes a store the engine already created — so the diesel
migrations are the engine's own, not this tool's guess at them — and replaces
the session rows with what came back from the other engine.

Copying rather than creating is the whole point. `wa-store-migrate`'s README
offers to apply the migrations from Node and INSERT by hand; that makes this
tool a second, silent owner of a schema it does not control, and the failure
mode is a store that opens and is wrong. A store the engine wrote has the schema
the engine expects, and the only thing changed here is rows.

The encodings are the inverses of the ones `dump-rust-store.py` reads, and each
is written next to the reason it is not the obvious thing:

- `prekeys.key` is a `PreKeyRecordStructure` protobuf, not a 64-byte pair.
- device keypairs are private-then-public, the reverse of how they read out.
"""

import base64
import json
import shutil
import sqlite3
import sys


def raw(value):
    return None if value is None else base64.b64decode(value)


def join_keypair(pair):
    """Private first, then public — the order `serialize_keypair` writes."""
    return raw(pair["privKey"]) + raw(pair["pubKey"])


def write_varint(value, out):
    while True:
        byte = value & 0x7F
        value >>= 7
        out.append(byte | 0x80 if value else byte)
        if not value:
            return


def prekey_record(key_id, pair):
    """`PreKeyRecordStructure`: 1 = id, 2 = public key, 3 = private key."""
    out = bytearray()
    out.append(0x08)  # field 1, varint
    write_varint(key_id, out)
    for number, blob in ((2, raw(pair["pubKey"])), (3, raw(pair["privKey"]))):
        out.append(number << 3 | 2)
        write_varint(len(blob), out)
        out.extend(blob)
    return bytes(out)


def write(template, target, snapshot, device_id=1):
    shutil.copyfile(template, target)
    # A copied SQLite file can carry a `-wal` its journal still refers to.
    for suffix in ("-wal", "-shm"):
        try:
            shutil.copyfile(f"{template}{suffix}", f"{target}{suffix}")
        except FileNotFoundError:
            pass

    connection = sqlite3.connect(target)
    connection.execute("PRAGMA foreign_keys = ON")
    device = snapshot["device"]

    connection.execute(
        """UPDATE device SET
               registration_id = ?, noise_key = ?, identity_key = ?,
               signed_pre_key = ?, signed_pre_key_id = ?,
               signed_pre_key_signature = ?, adv_secret_key = ?, account = ?,
               pn = ?, lid = ?, push_name = ?, next_pre_key_id = ?,
               server_has_prekeys = ?
           WHERE id = ?""",
        (
            device["registrationId"],
            join_keypair(device["noiseKey"]),
            join_keypair(device["identityKey"]),
            join_keypair(device["signedPreKey"]),
            device["signedPreKeyId"],
            raw(device["signedPreKeySignature"]),
            raw(device["advSecretKey"]),
            raw(device.get("account")),
            device.get("pn") or "",
            device.get("lid") or "",
            device.get("pushName") or "",
            device.get("nextPreKeyId") or 1,
            1 if device.get("serverHasPrekeys") else 0,
            device_id,
        ),
    )

    # Replaced wholesale rather than merged. A merge would leave behind rows the
    # other engine dropped — a prekey it handed out and deleted would come back
    # from the dead here, and the server would refuse the second use.
    for table in ("prekeys", "identities", "sessions", "sender_keys", "app_state_keys"):
        connection.execute(f"DELETE FROM {table} WHERE device_id = ?", (device_id,))

    connection.executemany(
        "INSERT INTO prekeys (id, key, uploaded, device_id) VALUES (?, ?, ?, ?)",
        [
            (
                record["keyId"],
                prekey_record(record["keyId"], record["keyPair"]),
                1 if record.get("uploaded") else 0,
                device_id,
            )
            for record in snapshot.get("preKeys", [])
        ],
    )

    connection.executemany(
        "INSERT INTO identities (address, key, device_id) VALUES (?, ?, ?)",
        [
            (entry["address"], raw(entry["key"]), device_id)
            for entry in snapshot.get("identities", [])
        ],
    )

    connection.executemany(
        "INSERT INTO sessions (address, record, device_id) VALUES (?, ?, ?)",
        [
            (entry["address"], raw(entry["record"]), device_id)
            for entry in snapshot.get("sessions", [])
        ],
    )

    connection.executemany(
        "INSERT INTO sender_keys (address, record, device_id) VALUES (?, ?, ?)",
        [
            (entry["address"], raw(entry["record"]), device_id)
            for entry in snapshot.get("senderKeys", [])
        ],
    )

    connection.executemany(
        "INSERT INTO app_state_keys (key_id, key_data, device_id) VALUES (?, ?, ?)",
        [
            (raw(entry["keyId"]), raw(entry["keyData"]), device_id)
            for entry in snapshot.get("appStateKeys", [])
        ],
    )

    # App-state versions are left as the template had them. The IR calls this
    # domain lossy in both directions and the round trip returned it byte for
    # byte, so rewriting it would be churn — and `state_data` is a bincode
    # `HashState` this tool has no reason to learn to encode.

    connection.commit()
    counts = {
        table: connection.execute(
            f"SELECT count(*) FROM {table} WHERE device_id = ?", (device_id,)
        ).fetchone()[0]
        for table in ("prekeys", "identities", "sessions", "sender_keys", "app_state_keys")
    }
    connection.close()
    return counts


if __name__ == "__main__":
    if len(sys.argv) < 4:
        raise SystemExit("usage: write-rust-store.py <template.db> <target.db> <snapshot.json>")
    with open(sys.argv[3], encoding="utf-8") as handle:
        data = json.load(handle)
    print(json.dumps(write(sys.argv[1], sys.argv[2], data)))
