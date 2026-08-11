/**
 * Fill a fresh `zapo` store with a session `zapo` did not create.
 *
 * This is the half of a handoff the migrator does not do. `wa-store-migrate`
 * turns one engine's snapshot into another's *shape*; putting that shape into a
 * running engine is the host's job, and deliberately so — D-007 says the host
 * never owns the store, which cuts both ways: nothing in `wa-wire` may reach
 * into an engine's persistence, so an attach is written against the engine's own
 * public contracts and nothing else.
 *
 * Every write below is a documented `zapo` store method, and every shape was
 * read off `zapo`'s own types rather than guessed. Two of them do not line up
 * with the migrator's output and are converted here:
 *
 * - `indexValueMap` is a plain object in a `ZapoAppStateCollectionVersion` and a
 *   `ReadonlyMap` in `WaAppStateCollectionStateUpdate`.
 * - `appState` arrives as `{keys, collections}` keyed by collection name; the
 *   store takes a flat list of updates that each name their collection.
 *
 * Records that are already in `zapo`'s shape — session records, sender-key
 * records — are passed through untouched. The migrator decoded them into that
 * shape on purpose, and re-reading them here would be a second opinion nobody
 * asked for.
 */

/** The address shape `zapo`'s signal stores take. */
function address(value) {
    return {
        user: value.user,
        // An absent server is `zapo`'s own default. Passing undefined rather
        // than a guess keeps the default where the engine put it.
        ...(value.server === undefined ? {} : { server: value.server }),
        device: value.device ?? 0,
    }
}

/**
 * Seed `session` from a `ZapoStoreSnapshot`, and report what was written.
 *
 * The counts come back so a caller can say what landed rather than assume: an
 * attach that silently wrote nothing looks exactly like one that worked, right
 * up until the engine cannot decrypt anything.
 */
export async function seed(session, snapshot) {
    const written = {}

    await session.auth.save(snapshot.credentials)
    written.credentials = 1

    const preKeys = snapshot.preKeys ?? []
    for (const record of preKeys) {
        // `PreKeyRecord` is `{keyId, keyPair, uploaded?}` and `SignalKeyPair` is
        // `{pubKey, privKey}` — the same names the migrator emits.
        await session.preKey.putPreKey(record)
    }
    // Whether the server already holds them is part of the session, not a
    // default: a client that re-uploads a full batch on attach tells the server
    // to throw away keys its peers may already be using.
    if (snapshot.credentials.serverHasPreKeys !== undefined) {
        await session.preKey.setServerHasPreKeys(snapshot.credentials.serverHasPreKeys)
    }
    written.preKeys = preKeys.length

    const identities = snapshot.identities ?? []
    if (identities.length > 0) {
        await session.identity.setRemoteIdentities(
            identities.map((entry) => ({
                address: address(entry.address),
                identityKey: entry.identityKey,
            }))
        )
    }
    written.identities = identities.length

    const sessions = snapshot.sessions ?? []
    if (sessions.length > 0) {
        await session.session.setSessionsBatch(
            sessions.map((entry) => ({
                address: address(entry.address),
                session: entry.record,
            }))
        )
    }
    written.sessions = sessions.length

    // Untested by the fixture, which holds no groups. Written against
    // `WaSenderKeyStore.upsertSenderKey`, whose `SenderKeyRecord` already
    // carries its own `groupId` and `sender` — the migrator hands those back
    // beside the record as well, and the record's own copies are the ones the
    // store indexes on.
    const senderKeys = snapshot.senderKeys ?? []
    for (const entry of senderKeys) {
        await session.senderKey.upsertSenderKey(entry.record)
    }
    written.senderKeys = senderKeys.length

    const keys = snapshot.appState?.keys ?? []
    if (keys.length > 0) {
        await session.appState.upsertSyncKeys(keys)
    }
    written.appStateSyncKeys = keys.length

    const collections = Object.entries(snapshot.appState?.collections ?? {})
    if (collections.length > 0) {
        await session.appState.setCollectionStates(
            collections.map(([collection, state]) => ({
                collection,
                version: state.version,
                hash: state.hash,
                indexValueMap: new Map(Object.entries(state.indexValueMap ?? {})),
            }))
        )
    }
    written.appStateVersions = collections.length

    return written
}
