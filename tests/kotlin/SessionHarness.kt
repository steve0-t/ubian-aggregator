package sk.cestou.platform

import java.util.concurrent.Executor
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private class SessionKeys : VaultKeys {
    var key: SecretKey? = null
    var available = true
    var invalidated = false
    var deleteFails = false
    var repeatedNonce: ByteArray? = null
    var created = 0
    override fun authenticationAvailable() = available
    override fun exists() = key != null
    override fun create() {
        created++
        key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
    }
    private fun usable(): SecretKey {
        if (invalidated) throw VaultException(VaultFailure.KEY_INVALIDATED)
        return key ?: throw VaultException(VaultFailure.KEY_MISSING)
    }
    override fun encryptCipher() = Cipher.getInstance("AES/GCM/NoPadding").apply {
        val nonce = repeatedNonce
        if (nonce == null) init(Cipher.ENCRYPT_MODE, usable())
        else init(Cipher.ENCRYPT_MODE, usable(), GCMParameterSpec(128, nonce))
    }
    override fun decryptCipher(nonce: ByteArray) = Cipher.getInstance("AES/GCM/NoPadding").apply {
        init(Cipher.DECRYPT_MODE, usable(), GCMParameterSpec(128, nonce))
    }
    override fun delete() {
        if (deleteFails) throw VaultException(VaultFailure.IO_FAILURE)
        key = null
    }
}

private class SessionMemoryFiles : VaultFiles {
    var record: ByteArray? = null
    var readFails = false
    var writeFails = false
    var deleteFails = false
    override fun read(): ByteArray? {
        if (readFails) throw VaultException(VaultFailure.IO_FAILURE)
        return record?.clone()
    }
    override fun write(record: ByteArray) {
        if (writeFails) throw VaultException(VaultFailure.IO_FAILURE)
        this.record = record.clone()
    }
    override fun delete() {
        if (deleteFails) throw VaultException(VaultFailure.IO_FAILURE)
        record = null
    }
}

private class SessionApproval : VaultAuthorization {
    var delayed = false
    var cancelled = false
    var substitute: Cipher? = null
    var finish: (() -> Unit)? = null
    override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
        val callback = {
            if (cancelled) complete(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
            else complete(Result.success(substitute ?: cipher))
        }
        if (delayed) finish = callback else callback()
        return { cancelled = true }
    }
}

private fun ensure(condition: Boolean, message: () -> String = { "Native session harness assertion failed" }) {
    if (!condition) throw AssertionError(message())
}

private fun sessionFailure(result: Result<*>?, expected: VaultFailure) {
    ensure((result?.exceptionOrNull() as? VaultException)?.failure == expected) {
        "Unexpected native session storage failure"
    }
}

fun main() {
    val registrations = SessionGenerationFence()
    ensure(registrations.cancel(10))
    registrations.accept(10) // Rust cancels before beginning this same epoch.
    ensure(registrations.cancel(11))
    try { registrations.accept(10); error("Canceled session invoke was registered") }
    catch (error: VaultException) { ensure(error.failure == VaultFailure.AUTHENTICATION_CANCELLED) }
    registrations.accept(11)
    ensure(!registrations.cancel(10)) // An old cancel cannot kill newer work.
    registrations.revoke() // Activity stop/replacement also fences registration.
    registrations.revoke() // Repeated lifecycle calls do not advance forever.
    try { registrations.accept(11); error("Stopped Activity invoke was registered") }
    catch (error: VaultException) { ensure(error.failure == VaultFailure.AUTHENTICATION_CANCELLED) }
    registrations.accept(12)
    val direct = Executor { it.run() }
    val keys = SessionKeys()
    val files = SessionMemoryFiles()
    val approval = SessionApproval()
    val vault = RealSessionVault(keys, files, { approval }, direct)
    val fixture = "{\"synthetic\":\"session fixture only\"}".toByteArray()
    var saved: Result<Unit>? = null
    val save: (Result<Unit>) -> Unit = { saved = it }
    val input = fixture.clone()
    vault.store(input, AppLanguage.EN, save)
    ensure(saved?.isSuccess == true && input.all { it == 0.toByte() })
    ensure(files.record != null && !files.record!!.contentEquals(fixture))
    ensure(vault.status().recordPresent && vault.status().availability == "available")

    var delivered: ByteArray? = null
    vault.restore(AppLanguage.UK) {
        delivered = it.getOrThrow()
        ensure(it.getOrThrow().contentEquals(fixture))
    }
    ensure(delivered!!.all { it == 0.toByte() })
    vault.restore(AppLanguage.EN) {
        delivered = it.getOrThrow()
        throw IllegalStateException("Synthetic callback failure")
    }
    ensure(delivered!!.all { it == 0.toByte() })

    val nonces = mutableSetOf<List<Byte>>()
    repeat(32) {
        vault.store(fixture.clone(), AppLanguage.SK, save)
        ensure(saved?.isSuccess == true)
        nonces.add(SessionRecord.nonce(files.record!!).toList())
    }
    ensure(nonces.size == 32)
    val committed = files.record!!.clone()
    files.record = committed.clone().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
    vault.restore(AppLanguage.SK) { sessionFailure(it, VaultFailure.TAMPERED) }
    files.record = committed.clone().also { it[10] = (it[10].toInt() xor 1).toByte() }
    vault.restore(AppLanguage.SK) { sessionFailure(it, VaultFailure.TAMPERED) }
    files.record = committed.clone().also { it[7] = 2 }
    vault.restore(AppLanguage.SK) { sessionFailure(it, VaultFailure.UNSUPPORTED_VERSION) }
    files.record = committed.clone().also { it[8] = 1 }
    vault.restore(AppLanguage.SK) { sessionFailure(it, VaultFailure.INVALID_RECORD) }
    ensure(vault.status().recordPresent && vault.status().availability == "invalid_record")
    files.record = committed.clone()

    keys.repeatedNonce = SessionRecord.nonce(committed)
    vault.store(fixture.clone(), AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.NONCE_REUSED)
    ensure(files.record!!.contentEquals(committed))
    keys.repeatedNonce = null
    approval.substitute = keys.encryptCipher()
    vault.store(fixture.clone(), AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.AUTHENTICATION_CANCELLED)
    approval.substitute = null
    keys.available = false
    vault.store(fixture.clone(), AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.AUTHENTICATION_UNAVAILABLE)
    keys.available = true
    keys.invalidated = true
    vault.restore(AppLanguage.EN) { sessionFailure(it, VaultFailure.KEY_INVALIDATED) }
    keys.invalidated = false
    files.writeFails = true
    vault.store(fixture.clone(), AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.IO_FAILURE)
    ensure(files.record!!.contentEquals(committed))
    files.writeFails = false
    files.readFails = true
    ensure(vault.status().availability == "io_failure" && vault.status().recordPresent)
    files.readFails = false
    keys.key = null
    val keyCreations = keys.created
    vault.store(fixture.clone(), AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.KEY_MISSING)
    ensure(keys.created == keyCreations && keys.key == null)
    ensure(vault.delete().isSuccess)

    approval.delayed = true
    var callbacks = 0
    val pending = fixture.clone()
    vault.store(pending, AppLanguage.EN) { saved = it; callbacks++ }
    val late = approval.finish!!
    vault.restore(AppLanguage.EN) { sessionFailure(it, VaultFailure.BUSY) }
    ensure(vault.delete().isSuccess)
    ensure(pending.all { it == 0.toByte() })
    sessionFailure(saved, VaultFailure.AUTHENTICATION_CANCELLED)
    late()
    ensure(callbacks == 1 && files.record == null && keys.key == null)

    approval.cancelled = false
    var reentrant: Result<Unit>? = null
    vault.store(fixture.clone(), AppLanguage.EN) {
        vault.store(fixture.clone(), AppLanguage.EN) { nested -> reentrant = nested }
    }
    val obsolete = approval.finish!!
    ensure(vault.delete().isSuccess)
    sessionFailure(reentrant, VaultFailure.BUSY)
    obsolete()
    ensure(files.record == null && keys.key == null)

    approval.delayed = false
    approval.cancelled = false
    vault.store(fixture.clone(), AppLanguage.EN, save)
    keys.deleteFails = true
    sessionFailure(vault.delete(), VaultFailure.IO_FAILURE)
    ensure(files.record == null)
    keys.deleteFails = false
    ensure(vault.delete().isSuccess)
    vault.store(fixture.clone(), AppLanguage.EN, save)
    files.deleteFails = true
    sessionFailure(vault.delete(), VaultFailure.IO_FAILURE)
    ensure(keys.key == null)
    files.deleteFails = false
    ensure(vault.delete().isSuccess)

    // A queued crypto completion must not write after cancel, even when prompt
    // authorization was already successful before the worker was scheduled.
    val queue = mutableListOf<Runnable>()
    val queued = RealSessionVault(keys, files, { approval }, Executor { queue.add(it) })
    var queuedCallbacks = 0
    val queuedInput = fixture.clone()
    queued.store(queuedInput, AppLanguage.EN) { queuedCallbacks++; saved = it }
    ensure(queue.size == 1)
    ensure(queued.delete().isSuccess)
    queue.removeAt(0).run()
    sessionFailure(saved, VaultFailure.AUTHENTICATION_CANCELLED)
    ensure(queuedCallbacks == 1 && files.record == null && keys.key == null)
    ensure(queuedInput.all { it == 0.toByte() })

    for (record in listOf(ByteArray(0), ByteArray(37), ByteArray(SessionRecord.MAX_RECORD + 1))) {
        try { SessionRecord.nonce(record); error("Invalid session record accepted") }
        catch (error: VaultException) { ensure(error.failure == VaultFailure.INVALID_RECORD) }
    }
    val oversized = ByteArray(SessionRecord.MAX_PLAINTEXT + 1) { 1 }
    vault.store(oversized, AppLanguage.EN, save)
    sessionFailure(saved, VaultFailure.INVALID_RECORD)
    ensure(oversized.all { it == 0.toByte() })
    println("Native session storage: codec, bounds, tamper, nonce variation, key loss/invalidation, explicit authorization, input wiping, callback failures, failed writes and cancel/delete races passed (JVM substitutes, not Android Keystore).")
}
