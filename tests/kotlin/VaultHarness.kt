package sk.cestou.platform

import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private class MemoryKeys : VaultKeys {
    var key: SecretKey? = null
    var invalidated = false
    var available = true
    var deletionFails = false
    override fun authenticationAvailable() = available
    override fun exists() = key != null
    override fun create() { key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey() }
    private fun requireKey(): SecretKey {
        if (invalidated) throw VaultException(VaultFailure.KEY_INVALIDATED)
        return key ?: throw VaultException(VaultFailure.KEY_MISSING)
    }
    override fun encryptCipher(): Cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, requireKey()) }
    override fun decryptCipher(nonce: ByteArray): Cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, requireKey(), GCMParameterSpec(128, nonce)) }
    override fun delete() {
        if (deletionFails) throw VaultException(VaultFailure.IO_FAILURE)
        key = null
    }
}

private class MemoryFiles : VaultFiles {
    var record: ByteArray? = null
    var writeFails = false
    var deleteFails = false
    override fun read() = record?.clone()
    override fun write(record: ByteArray) {
        if (writeFails) throw VaultException(VaultFailure.IO_FAILURE)
        this.record = record.clone()
    }
    override fun delete() {
        if (deleteFails) throw VaultException(VaultFailure.IO_FAILURE)
        record = null
    }
}

private class Authorization : VaultAuthorization {
    var cancelled = false
    var delayed = false
    var finish: (() -> Unit)? = null
    override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
        val callback = {
            if (cancelled) complete(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
            else complete(Result.success(cipher))
        }
        if (delayed) finish = callback else callback()
        return { cancelled = true }
    }
}

private fun failure(result: Result<Unit>?, expected: VaultFailure) {
    check((result?.exceptionOrNull() as? VaultException)?.failure == expected) { "Unexpected synthetic vault status" }
}

fun main() {
    val keys = MemoryKeys()
    val files = MemoryFiles()
    val auth = Authorization()
    val vault = SyntheticVault(keys, files, auth)
    var result: Result<Unit>? = null
    val done: (Result<Unit>) -> Unit = { result = it }
    vault.saveOrUpdateSynthetic(done)
    check(result?.isSuccess == true && keys.exists() && files.record != null)
    check(!vault.availability().realCredentialsEnabled)
    vault.unlockAndVerifySynthetic(done)
    check(result?.isSuccess == true)
    val nonces = mutableSetOf<List<Byte>>()
    repeat(32) {
        vault.saveOrUpdateSynthetic(done)
        check(result?.isSuccess == true)
        nonces.add(VaultRecord.nonce(files.record!!).toList())
    }
    check(nonces.size == 32)
    val original = files.record!!.clone()
    files.record!![files.record!!.lastIndex] = (files.record!!.last().toInt() xor 1).toByte()
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.TAMPERED)
    files.record = original.clone().also { it[8] = 2 }
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.INVALID_RECORD)
    files.record = original.clone().also { it[7] = 2 }
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.UNSUPPORTED_VERSION)
    files.record = original.clone()
    auth.cancelled = true
    vault.saveOrUpdateSynthetic(done)
    failure(result, VaultFailure.AUTHENTICATION_CANCELLED)
    check(files.record!!.contentEquals(original))
    auth.cancelled = false
    keys.invalidated = true
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.KEY_INVALIDATED)
    keys.invalidated = false
    keys.key = null
    vault.saveOrUpdateSynthetic(done)
    failure(result, VaultFailure.KEY_MISSING)
    check(keys.key == null) // Existing ciphertext never causes silent key replacement.
    check(vault.availability().availability == "key_missing")
    check(vault.deleteSynthetic().isSuccess)
    check(keys.key == null && files.record == null)
    vault.saveOrUpdateSynthetic(done)
    val committed = files.record!!.clone()
    files.writeFails = true
    vault.saveOrUpdateSynthetic(done)
    failure(result, VaultFailure.IO_FAILURE)
    check(files.record!!.contentEquals(committed))
    files.writeFails = false
    vault.unlockAndVerifySynthetic(done)
    check(result?.isSuccess == true)
    keys.available = false
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.AUTHENTICATION_UNAVAILABLE)
    keys.available = true
    auth.delayed = true
    var callbacks = 0
    vault.saveOrUpdateSynthetic { result = it; callbacks++ }
    val late = auth.finish!!
    vault.unlockAndVerifySynthetic(done)
    failure(result, VaultFailure.BUSY)
    check(vault.deleteSynthetic().isSuccess)
    failure(result, VaultFailure.AUTHENTICATION_CANCELLED)
    late()
    check(callbacks == 1 && files.record == null && keys.key == null)
    // Deletion tries both steps and stays retryable on partial failure.
    auth.delayed = false
    auth.cancelled = false
    vault.saveOrUpdateSynthetic(done)
    files.deleteFails = true
    failure(vault.deleteSynthetic(), VaultFailure.IO_FAILURE)
    check(keys.key == null && files.record != null)
    files.deleteFails = false
    check(vault.deleteSynthetic().isSuccess)
    vault.saveOrUpdateSynthetic(done)
    keys.deletionFails = true
    failure(vault.deleteSynthetic(), VaultFailure.IO_FAILURE)
    check(files.record == null)
    keys.deletionFails = false
    check(vault.deleteSynthetic().isSuccess)
    for (record in listOf(ByteArray(0), ByteArray(36), ByteArray(VaultRecord.MAX_RECORD + 1))) {
        try { VaultRecord.nonce(record); error("Invalid record accepted") }
        catch (error: VaultException) { check(error.failure == VaultFailure.INVALID_RECORD) }
    }
    check(AppLanguage.parse("uk")?.text("sk", "en", "uk") == "uk")
    check(AppLanguage.parse("xx") == null)
    println("Synthetic vault: round-trip, tamper, nonces, key loss/invalidation, auth cancellation, failed replacement and deletion races passed (JVM substitutes, not Android Keystore).")
}
