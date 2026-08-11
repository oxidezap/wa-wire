/**
 * Read a session back out of a running `zapo` store, through its own contracts.
 *
 * The earlier note here said `zapo` could be attached to from outside and not
 * harvested from outside. That was half right, and the half that was wrong is
 * the important one: `auth.load()` returns the whole credential record and
 * `appState.exportData()` is a bulk read, so the volatile parts come back
 * whole. What the contracts genuinely lack is *discovery* — `getPreKeysById`,
 * `getSessionsBatch` and `getRemoteIdentities` each answer about keys you
 * already name, and nothing lists what is there.
 *
 * So a harvest has to bound what it might not know about, rather than hope:
 *
 * - **Prekeys.** Ask for every id that was seeded, then keep probing past the
 *   highest until a run of misses. A client that generated more during the leg
 *   numbered them upward from where it found the store.
 * - **Sessions and identities.** A new peer only appears if a stanza came from
 *   one, and the leg's own recording is the list of stanzas. Every JID that
 *   appears in the traffic is asked for, on top of what was seeded.
 *
 * That leaves one gap named rather than papered over: a session written for a
 * peer that never appears in the recorded traffic would be missed. Nothing in
 * the protocol writes one — a session comes from a message, a retry or a
 * prekey fetch, and all three are stanzas — but it is an argument, not a
 * guarantee, and the difference matters enough to write down.
 */

/** How many consecutive absent ids end the probe past the seeded range. */
const PROBE_RUN = 32

/**
 * Every prekey the store still holds: the seeded ids, plus any numbered above
 * them.
 */
async function harvestPreKeys(session, seededIds) {
    const found = []

    if (seededIds.length > 0) {
        const held = await session.preKey.getPreKeysById(seededIds)
        for (const record of held) {
            if (record) found.push(record)
        }
    }

    // A leg that generated keys numbered them upward from the highest it found.
    let id = seededIds.length > 0 ? Math.max(...seededIds) + 1 : 1
    let misses = 0
    while (misses < PROBE_RUN) {
        // eslint-disable-next-line no-await-in-loop
        const record = await session.preKey.getPreKeyById(id)
        if (record) {
            found.push(record)
            misses = 0
        } else {
            misses += 1
        }
        id += 1
    }

    return found
}

/** `user[:device]@server` for the addresses the contracts take. */
function addressKey(address) {
    return `${address.user}|${address.server ?? ''}|${address.device ?? 0}`
}

/**
 * Ask for every address that was seeded and every one the traffic mentioned.
 *
 * Deduplicated on the way in: asking twice is harmless and reporting twice is
 * not, since a caller counts these to decide whether anything moved.
 */
async function harvestSignal(session, seeded, fromTraffic) {
    const addresses = new Map()
    for (const address of [...seeded, ...fromTraffic]) {
        addresses.set(addressKey(address), address)
    }
    const list = [...addresses.values()]
    if (list.length === 0) return { sessions: [], identities: [] }

    const [records, keys] = await Promise.all([
        session.session.getSessionsBatch(list),
        session.identity.getRemoteIdentities(list),
    ])

    const sessions = []
    const identities = []
    list.forEach((address, index) => {
        if (records[index]) sessions.push({ address, record: records[index] })
        if (keys[index]) identities.push({ address, identityKey: keys[index] })
    })
    return { sessions, identities }
}

/**
 * Read the session back as a `ZapoStoreSnapshot`.
 *
 * `seeded` is what went in, and is not decoration: it is how the harvest knows
 * which ids and addresses to ask about, and a harvest given the wrong one comes
 * back quietly short.
 */
export async function harvest(session, seeded, addressesInTraffic = []) {
    const credentials = await session.auth.load()
    if (!credentials) {
        throw new Error('the store has no credentials — nothing was attached, or it was cleared')
    }

    const seededIds = (seeded.preKeys ?? []).map((record) => record.keyId)
    const seededAddresses = [
        ...(seeded.sessions ?? []).map((entry) => entry.address),
        ...(seeded.identities ?? []).map((entry) => entry.address),
    ]

    const [preKeys, signal, appState] = await Promise.all([
        harvestPreKeys(session, seededIds),
        harvestSignal(session, seededAddresses, addressesInTraffic),
        session.appState.exportData(),
    ])

    return {
        credentials,
        preKeys,
        sessions: signal.sessions,
        identities: signal.identities,
        // Sender keys are group state and this route carries none; a group leg
        // would need the group ids, which have the same discovery problem and
        // the same answer — they are in the traffic.
        senderKeys: [],
        // `WaAppStateStoreData` is already `{keys, collections}` — the shape a
        // `ZapoStoreSnapshot` wants, so this is a pass-through rather than a
        // translation with somewhere to go wrong.
        appState,
    }
}
