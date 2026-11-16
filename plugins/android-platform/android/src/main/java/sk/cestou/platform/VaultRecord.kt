package sk.cestou.platform

import java.nio.ByteBuffer
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher

internal enum class VaultFailure {
    AUTHENTICATION_UNAVAILABLE, AUTHENTICATION_CANCELLED, KEY_MISSING,
    KEY_INVALIDATED, TAMPERED, UNSUPPORTED_VERSION, INVALID_RECORD,
    IO_FAILURE, BUSY, NONCE_REUSED,
}

internal class VaultException(val failure: VaultFailure) : Exception(failure.name.lowercase())

/** V1: magic(4), version(4 BE), purpose(1), nonce(12), ciphertext + GCM tag(16).
 * The complete 9-byte header is authenticated. Only synthetic purpose 1 exists.
 * Unknown versions require a future explicit, authenticated migration. */
internal object VaultRecord {
    const val MAX_PLAINTEXT = 4096
    const val MAX_RECORD = 9 + 12 + MAX_PLAINTEXT + 16
    private val header = ByteBuffer.allocate(9).putInt(0x43564155).putInt(1).put(1).array()

    fun nonce(record: ByteArray): ByteArray {
        if (record.size !in 37..MAX_RECORD) throw VaultException(VaultFailure.INVALID_RECORD)
        val buffer = ByteBuffer.wrap(record)
        if (buffer.int != 0x43564155) throw VaultException(VaultFailure.INVALID_RECORD)
        if (buffer.int != 1) throw VaultException(VaultFailure.UNSUPPORTED_VERSION)
        if (buffer.get().toInt() != 1) throw VaultException(VaultFailure.INVALID_RECORD)
        return record.copyOfRange(9, 21)
    }

    fun seal(authorizedCipher: Cipher, synthetic: ByteArray): ByteArray {
        if (synthetic.size > MAX_PLAINTEXT || authorizedCipher.iv.size != 12) {
            throw VaultException(VaultFailure.INVALID_RECORD)
        }
        authorizedCipher.updateAAD(header)
        return header + authorizedCipher.iv + authorizedCipher.doFinal(synthetic)
    }

    fun verify(authorizedCipher: Cipher, record: ByteArray, synthetic: ByteArray): Boolean {
        nonce(record)
        authorizedCipher.updateAAD(record.copyOfRange(0, 9))
        val plaintext = try { authorizedCipher.doFinal(record, 21, record.size - 21) }
        catch (_: AEADBadTagException) { throw VaultException(VaultFailure.TAMPERED) }
        return try { java.security.MessageDigest.isEqual(plaintext, synthetic) }
        finally { plaintext.fill(0) }
    }
}

internal interface VaultKeys {
    fun authenticationAvailable(): Boolean
    fun exists(): Boolean
    fun create()
    fun encryptCipher(): Cipher
    fun decryptCipher(nonce: ByteArray): Cipher
    fun delete()
    fun failure(error: Throwable): VaultException = error as? VaultException ?: VaultException(VaultFailure.IO_FAILURE)
}

internal interface VaultFiles {
    fun read(): ByteArray?
    fun write(record: ByteArray)
    fun delete()
}

internal interface VaultAuthorization {
    // The returned cancellation action must stop the prompt. The callback may
    // still arrive; the engine's generation check rejects it after deletion.
    fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit
}

internal data class VaultMetadata(val availability: String, val syntheticRecordPresent: Boolean) {
    val realCredentialsEnabled = false
}

/** Synthetic-only account preparation. No general secret input/read/use API.
 * All instances must be owned by one coordinator per record. Synchronized
 * transitions fence cancellation, deletion and late authentication callbacks. */
internal class SyntheticVault(
    private val keys: VaultKeys,
    private val files: VaultFiles,
    private val authorization: VaultAuthorization,
) {
    private var generation = 0L
    private var pending: ((Result<Unit>) -> Unit)? = null
    private var cancelPrompt: (() -> Unit)? = null
    private var deleting = false

    @Synchronized fun availability(): VaultMetadata = try {
        val record = files.read()
        record?.let { VaultRecord.nonce(it) }
        VaultMetadata(when {
            record != null && !keys.exists() -> "key_missing"
            !keys.authenticationAvailable() -> "authentication_unavailable"
            else -> "available"
        }, record != null)
    } catch (error: Exception) {
        val availability = when (safeFailure(error).failure) {
            VaultFailure.KEY_INVALIDATED -> "key_invalidated"
            VaultFailure.INVALID_RECORD, VaultFailure.UNSUPPORTED_VERSION, VaultFailure.TAMPERED -> "invalid_record"
            else -> "io_failure"
        }
        VaultMetadata(availability, false)
    }

    fun saveOrUpdateSynthetic(complete: (Result<Unit>) -> Unit) = start(true, complete)
    fun unlockAndVerifySynthetic(complete: (Result<Unit>) -> Unit) = start(false, complete)

    @Synchronized private fun start(save: Boolean, complete: (Result<Unit>) -> Unit) {
        if (deleting || pending != null) { deliver(complete, Result.failure(VaultException(VaultFailure.BUSY))); return }
        try {
            if (!keys.authenticationAvailable()) throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
            val previous = files.read()
            previous?.let { VaultRecord.nonce(it) }
            if (!keys.exists()) {
                if (previous != null || !save) throw VaultException(VaultFailure.KEY_MISSING)
                keys.create()
            }
            if (!save && previous == null) throw VaultException(VaultFailure.INVALID_RECORD)
            val cipher = if (save) keys.encryptCipher() else keys.decryptCipher(VaultRecord.nonce(previous!!))
            val ticket = ++generation
            pending = complete
            val cancel = authorization.authorize(cipher) { result ->
                synchronized(this) {
                    if (ticket != generation || pending == null) return@synchronized
                    val outcome = runCatching {
                        val authorized = result.getOrThrow()
                        if (authorized !== cipher) throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
                        // Created only after authorization; never exposed to a caller.
                        val synthetic = "Cestou synthetic account storage v1".toByteArray(Charsets.UTF_8)
                        try {
                            if (save) {
                                if (previous != null && VaultRecord.nonce(previous).contentEquals(authorized.iv)) {
                                    throw VaultException(VaultFailure.NONCE_REUSED)
                                }
                                val sealed = VaultRecord.seal(authorized, synthetic)
                                files.write(sealed)
                            } else if (!VaultRecord.verify(authorized, previous!!, synthetic)) {
                                throw VaultException(VaultFailure.TAMPERED)
                            }
                        } finally { synthetic.fill(0) }
                    }.mapFailure()
                    val callback = pending
                    pending = null
                    cancelPrompt = null
                    deliver(callback, outcome)
                }
            }
            if (pending != null && ticket == generation) cancelPrompt = cancel
        } catch (error: Exception) {
            pending = null
            cancelPrompt = null
            deliver(complete, Result.failure(safeFailure(error)))
        }
    }

    @Synchronized fun cancelUnlock() {
        generation++
        val callback = pending
        pending = null
        val cancel = cancelPrompt
        cancelPrompt = null
        try { cancel?.invoke() } catch (_: Exception) { /* Generation already revoked. */ }
        try { callback?.invoke(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED))) }
        catch (_: Exception) { /* Cleanup must still delete the key and file. */ }
    }

    @Synchronized fun deleteSynthetic(): Result<Unit> {
        if (deleting) return Result.failure(VaultException(VaultFailure.BUSY))
        deleting = true
        try {
            cancelUnlock()
            // Key deletion first makes any stranded ciphertext unusable. Attempt
            // both removals even if one fails, and report failure for a safe retry.
            val keyResult = runCatching { keys.delete() }
            val fileResult = runCatching { files.delete() }
            return (keyResult.exceptionOrNull() ?: fileResult.exceptionOrNull())?.let {
                Result.failure(safeFailure(it))
            } ?: Result.success(Unit)
        } finally { deleting = false }
    }

    private fun safeFailure(error: Throwable): VaultException =
        keys.failure(error)

    private fun deliver(callback: ((Result<Unit>) -> Unit)?, result: Result<Unit>) {
        try { callback?.invoke(result) } catch (_: Exception) { /* State is already settled. */ }
    }

    private fun Result<Unit>.mapFailure(): Result<Unit> = fold(
        { Result.success(Unit) }, { Result.failure(safeFailure(it)) },
    )
}
