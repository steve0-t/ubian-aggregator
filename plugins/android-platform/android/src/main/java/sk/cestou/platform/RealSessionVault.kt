package sk.cestou.platform

import android.app.Activity
import android.content.Context
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal
import android.os.Handler
import android.os.Looper
import android.system.Os
import android.util.AtomicFile
import android.view.WindowManager
import java.io.File
import java.nio.ByteBuffer
import java.lang.ref.WeakReference
import java.util.concurrent.Executor
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import java.util.WeakHashMap
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher

/** Independent authenticated format for an explicit saved native account session.
 * No password is part of the Rust-owned snapshot. It never uses the synthetic
 * account-preparation format, alias or file. Unknown versions fail closed. */
internal object SessionRecord {
    const val MAX_PLAINTEXT = 256 * 1024
    const val MAX_RECORD = 9 + 12 + MAX_PLAINTEXT + 16
    private const val MAGIC = 0x43534553 // CSES
    private const val VERSION = 1
    private const val PURPOSE: Byte = 2
    private val header = ByteBuffer.allocate(9).putInt(MAGIC).putInt(VERSION).put(PURPOSE).array()

    fun nonce(record: ByteArray): ByteArray {
        if (record.size !in 38..MAX_RECORD) throw VaultException(VaultFailure.INVALID_RECORD)
        val input = ByteBuffer.wrap(record)
        if (input.int != MAGIC) throw VaultException(VaultFailure.INVALID_RECORD)
        if (input.int != VERSION) throw VaultException(VaultFailure.UNSUPPORTED_VERSION)
        if (input.get() != PURPOSE) throw VaultException(VaultFailure.INVALID_RECORD)
        return record.copyOfRange(9, 21)
    }

    fun seal(cipher: Cipher, snapshot: ByteArray): ByteArray {
        if (snapshot.isEmpty() || snapshot.size > MAX_PLAINTEXT || cipher.iv.size != 12) {
            throw VaultException(VaultFailure.INVALID_RECORD)
        }
        cipher.updateAAD(header)
        return header + cipher.iv + cipher.doFinal(snapshot)
    }

    fun open(cipher: Cipher, record: ByteArray): ByteArray {
        nonce(record)
        cipher.updateAAD(record.copyOfRange(0, 9))
        val plaintext = try { cipher.doFinal(record, 21, record.size - 21) }
        catch (_: AEADBadTagException) { throw VaultException(VaultFailure.TAMPERED) }
        if (plaintext.isEmpty() || plaintext.size > MAX_PLAINTEXT) {
            plaintext.fill(0)
            throw VaultException(VaultFailure.INVALID_RECORD)
        }
        return plaintext
    }
}

/** Private, backup-excluded, atomic storage. Codec validation stays in the
 * coordinator so corrupt records remain visible and can be explicitly deleted. */
internal class SessionFiles(directory: File, name: String) : VaultFiles {
    private val file = AtomicFile(File(directory, name))

    override fun read(): ByteArray? = try {
        file.openRead().use { input ->
            val output = java.io.ByteArrayOutputStream()
            val chunk = ByteArray(4096)
            try {
                while (output.size() <= SessionRecord.MAX_RECORD) {
                    val count = input.read(chunk, 0, minOf(chunk.size, SessionRecord.MAX_RECORD + 1 - output.size()))
                    if (count < 0) break
                    output.write(chunk, 0, count)
                }
                output.toByteArray().also {
                    if (it.size > SessionRecord.MAX_RECORD) throw VaultException(VaultFailure.INVALID_RECORD)
                }
            } finally { chunk.fill(0) }
        }
    } catch (_: java.io.FileNotFoundException) {
        if (exists()) throw VaultException(VaultFailure.IO_FAILURE)
        null
    }

    private fun exists() = file.baseFile.exists() || File(file.baseFile.path + ".bak").exists() ||
        File(file.baseFile.path + ".new").exists()

    override fun write(record: ByteArray) {
        SessionRecord.nonce(record)
        val stream = file.startWrite()
        try {
            Os.fchmod(stream.fd, 384) // 0600
            stream.write(record)
            stream.fd.sync()
            file.finishWrite(stream)
            if (!record.contentEquals(read())) throw VaultException(VaultFailure.IO_FAILURE)
        } catch (_: Exception) {
            file.failWrite(stream)
            throw VaultException(VaultFailure.IO_FAILURE)
        }
    }

    override fun delete() {
        file.delete()
        if (exists()) throw VaultException(VaultFailure.IO_FAILURE)
    }
}

internal data class SessionStorageMetadata(val availability: String, val recordPresent: Boolean)

/** Rust's operation epoch covers cancellation delivered before an invoke has
 * registered here. Equal epochs are allowed for cancel-then-start; old cancels
 * cannot revoke a newer operation. Only process-owned storage uses this fence. */
internal class SessionGenerationFence {
    private var floor = 0L
    private var latest = 0L

    fun accept(generation: Long) {
        if (generation < 0 || generation < floor) throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        floor = generation
        latest = maxOf(latest, generation)
    }

    fun cancel(generation: Long): Boolean {
        if (generation < 0) throw VaultException(VaultFailure.INVALID_RECORD)
        if (generation < floor) return false
        accept(generation)
        return true
    }

    fun revoke() {
        val next = if (latest == Long.MAX_VALUE) Long.MAX_VALUE else latest + 1
        floor = maxOf(floor, next)
    }
}

/** One owner per session record. Bytes are consumed and wiped, and restored
 * bytes are only delivered to the internal native/Rust bridge during callback.
 * Explicit Save/Unlock authorizes each operation; there is no background save. */
internal class RealSessionVault(
    private val keys: VaultKeys,
    private val files: VaultFiles,
    private val authorization: (AppLanguage) -> VaultAuthorization,
    private val worker: Executor,
) {
    private var generation = 0L
    private var pending: ((Result<ByteArray?>) -> Unit)? = null
    private var cancelPrompt: (() -> Unit)? = null
    private var payload: ByteArray? = null
    private var deleting = false

    @Synchronized fun status(): SessionStorageMetadata {
        var present = true // An unreadable record must remain explicitly deletable.
        return try {
            val record = files.read()
            present = record != null
            record?.let { SessionRecord.nonce(it) }
            SessionStorageMetadata(when {
                present && !keys.exists() -> "key_missing"
                !keys.authenticationAvailable() -> "authentication_unavailable"
                else -> "available"
            }, present)
        } catch (error: Exception) {
            val availability = when (failure(error).failure) {
                VaultFailure.KEY_MISSING -> "key_missing"
                VaultFailure.KEY_INVALIDATED -> "key_invalidated"
                VaultFailure.INVALID_RECORD, VaultFailure.UNSUPPORTED_VERSION, VaultFailure.TAMPERED -> "invalid_record"
                else -> "io_failure"
            }
            SessionStorageMetadata(availability, present)
        }
    }

    fun store(snapshot: ByteArray, language: AppLanguage, complete: (Result<Unit>) -> Unit) {
        start(snapshot, language) { result -> complete(result.map { Unit }) }
    }

    fun restore(language: AppLanguage, complete: (Result<ByteArray>) -> Unit) {
        start(null, language) { result ->
            complete(result.mapCatching { it ?: throw VaultException(VaultFailure.INVALID_RECORD) })
        }
    }

    @Synchronized private fun start(snapshot: ByteArray?, language: AppLanguage, complete: (Result<ByteArray?>) -> Unit) {
        if (deleting || pending != null) {
            snapshot?.fill(0)
            deliver(complete, Result.failure(VaultException(VaultFailure.BUSY)))
            return
        }
        try {
            if (snapshot != null && (snapshot.isEmpty() || snapshot.size > SessionRecord.MAX_PLAINTEXT)) {
                throw VaultException(VaultFailure.INVALID_RECORD)
            }
            if (!keys.authenticationAvailable()) throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
            val previous = files.read()
            previous?.let { SessionRecord.nonce(it) }
            if (!keys.exists()) {
                if (previous != null || snapshot == null) throw VaultException(VaultFailure.KEY_MISSING)
                keys.create()
            }
            if (snapshot == null && previous == null) throw VaultException(VaultFailure.INVALID_RECORD)
            val cipher = if (snapshot != null) keys.encryptCipher() else keys.decryptCipher(SessionRecord.nonce(previous!!))
            val ticket = ++generation
            pending = complete
            payload = snapshot
            val cancel = authorization(language).authorize(cipher) { authorized ->
                try {
                    worker.execute {
                        synchronized(this) {
                            if (ticket != generation || pending == null) return@synchronized
                            val result = runCatching<ByteArray?> {
                                val approvedCipher = authorized.getOrThrow()
                                if (approvedCipher !== cipher) throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
                                if (snapshot != null) {
                                    if (previous != null && SessionRecord.nonce(previous).contentEquals(approvedCipher.iv)) {
                                        throw VaultException(VaultFailure.NONCE_REUSED)
                                    }
                                    files.write(SessionRecord.seal(approvedCipher, snapshot))
                                    null
                                } else SessionRecord.open(approvedCipher, previous!!)
                            }.fold({ Result.success(it) }, { Result.failure(failure(it)) })
                            val callback = pending
                            pending = null
                            cancelPrompt = null
                            payload?.fill(0)
                            payload = null
                            deliver(callback, result)
                        }
                    }
                } catch (_: Exception) { this.cancel() }
            }
            if (ticket == generation && pending != null) cancelPrompt = cancel
        } catch (error: Exception) {
            snapshot?.fill(0)
            payload = null
            pending = null
            cancelPrompt = null
            deliver(complete, Result.failure(failure(error)))
        }
    }

    @Synchronized fun cancel() {
        generation++
        val callback = pending
        val dismiss = cancelPrompt
        pending = null
        cancelPrompt = null
        payload?.fill(0)
        payload = null
        try { dismiss?.invoke() } catch (_: Exception) { /* Generation is already revoked. */ }
        deliver(callback, Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
    }

    @Synchronized fun delete(): Result<Unit> {
        if (deleting) return Result.failure(VaultException(VaultFailure.BUSY))
        deleting = true
        return try {
            cancel()
            val keyError = runCatching {
                keys.delete()
                if (keys.exists()) throw VaultException(VaultFailure.IO_FAILURE)
            }.exceptionOrNull()
            val fileError = runCatching { files.delete() }.exceptionOrNull()
            val error = keyError ?: fileError
            if (error == null) Result.success(Unit) else Result.failure(failure(error))
        } finally { deleting = false }
    }

    private fun failure(error: Throwable) = keys.failure(error)

    private fun deliver(callback: ((Result<ByteArray?>) -> Unit)?, result: Result<ByteArray?>) {
        try { callback?.invoke(result) }
        catch (_: Exception) { /* State is settled; no secret or exception is logged. */ }
        finally { result.getOrNull()?.fill(0) }
    }

}

/** A process owns exactly one coordinator for the session file/key. An Activity
 * gets a lease rather than another writer. Recreating the main Activity revokes
 * pending work before transferring the lease, and callbacks from the old
 * Activity cannot cancel or overwrite work started by its replacement. */
internal object NativeSessionStorage {
    private var owner = 0L
    private var activity = WeakReference<Activity>(null)
    private var application: Context? = null
    private var foreground = false
    private val generations = SessionGenerationFence()
    private var vault: RealSessionVault? = null
    private val worker = Executors.newSingleThreadExecutor()

    @Synchronized fun attach(next: Activity): SessionStorageLease {
        vault?.cancel()
        if (owner != 0L) generations.revoke()
        owner++
        activity = WeakReference(next)
        application = next.applicationContext
        foreground = true
        return SessionStorageLease(owner)
    }

    @Synchronized fun <T> use(lease: Long, operation: (RealSessionVault) -> T): T {
        if (lease != owner) throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        return operation(coordinator())
    }

    @Synchronized fun <T> start(lease: Long, generation: Long, operation: (RealSessionVault) -> T): T {
        val current = activity.get()
        if (lease != owner || !foreground || current == null || current.isFinishing || current.isDestroyed) {
            throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        }
        generations.accept(generation)
        return operation(coordinator())
    }

    @Synchronized fun delete(lease: Long, generation: Long): Result<Unit> {
        if (lease != owner) throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        generations.accept(generation)
        vault?.let { return it.delete() }
        // Forget must still remove ciphertext when opening Android Keystore
        // itself fails. Do not let coordinator construction skip file cleanup.
        val context = application ?: throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        val keyError = runCatching {
            val keys = AndroidVaultKeys(context, "cestou.native-session.v1")
            keys.delete()
            if (keys.exists()) throw VaultException(VaultFailure.IO_FAILURE)
        }.exceptionOrNull()
        val fileError = runCatching {
            SessionFiles(context.noBackupFilesDir, "native-session.v1.bin").delete()
        }.exceptionOrNull()
        val error = keyError ?: fileError
        return if (error == null) Result.success(Unit)
        else Result.failure(error as? VaultException ?: VaultException(VaultFailure.IO_FAILURE))
    }

    private fun coordinator(): RealSessionVault {
        val context = application ?: throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
        return vault ?: RealSessionVault(
            AndroidVaultKeys(context, "cestou.native-session.v1"),
            SessionFiles(context.noBackupFilesDir, "native-session.v1.bin"),
            { language ->
                val target = activity.get()
                    ?: throw VaultException(VaultFailure.AUTHENTICATION_CANCELLED)
                SessionAuthorization(target, language)
            },
            worker,
        ).also { vault = it }
    }

    @Synchronized fun cancel(lease: Long, generation: Long) {
        if (lease == owner && generations.cancel(generation)) {
            vault?.cancel()
        }
    }

    @Synchronized fun pause(lease: Long) {
        if (lease != owner) return
        foreground = false
        generations.revoke()
        vault?.cancel()
    }

    @Synchronized fun resume(lease: Long) {
        if (lease == owner) foreground = true
    }

}

internal class SessionStorageLease(private val owner: Long) {
    fun status() = NativeSessionStorage.use(owner) { it.status() }
    fun store(snapshot: ByteArray, language: AppLanguage, generation: Long, complete: (Result<Unit>) -> Unit) {
        try { NativeSessionStorage.start(owner, generation) { it.store(snapshot, language, complete) } }
        catch (error: Exception) { snapshot.fill(0); throw error }
    }
    fun restore(language: AppLanguage, generation: Long, complete: (Result<ByteArray>) -> Unit) =
        NativeSessionStorage.start(owner, generation) { it.restore(language, complete) }
    fun delete(generation: Long) = NativeSessionStorage.delete(owner, generation)
    fun cancel(generation: Long) = NativeSessionStorage.cancel(owner, generation)
    fun pause() = NativeSessionStorage.pause(owner)
    fun resume() = NativeSessionStorage.resume(owner)
}

/** Account UI and biometric prompts share ownership of the window flag. A
 * finishing prompt must never clear protection enabled by a still-open form. */
internal object AccountProtection {
    private data class Flags(val original: Boolean, var account: Boolean = false, var prompts: Int = 0)
    private val windows = WeakHashMap<Activity, Flags>()

    fun account(activity: Activity, enabled: Boolean) {
        val flags = flags(activity)
        flags.account = enabled
        apply(activity, flags)
    }

    fun prompt(activity: Activity): () -> Unit {
        val flags = flags(activity)
        flags.prompts++
        apply(activity, flags)
        var finished = false
        return {
            if (!finished) {
                finished = true
                flags.prompts--
                apply(activity, flags)
            }
        }
    }

    private fun flags(activity: Activity) = windows.getOrPut(activity) {
        Flags(activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
    }

    private fun apply(activity: Activity, flags: Flags) {
        if (flags.original || flags.account || flags.prompts > 0) activity.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        else activity.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
    }
}

/** The same Cipher CryptoObject must be authorized and used. Cancel/teardown
 * restores screenshot flags even if Android never delivers a final callback. */
internal class SessionAuthorization(private val activity: Activity, private val language: AppLanguage) : VaultAuthorization {
    override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
        if (Build.VERSION.SDK_INT < 29) throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
        val cancellation = CancellationSignal()
        val canceled = AtomicBoolean(false)
        val handler = Handler(Looper.getMainLooper())
        var finishOnUi: ((Result<Cipher>) -> Unit)? = null
        activity.runOnUiThread {
            if (activity.isFinishing || activity.isDestroyed || canceled.get()) {
                complete(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
                return@runOnUiThread
            }
            val releaseProtection = try { AccountProtection.prompt(activity) }
            catch (_: Exception) {
                complete(Result.failure(VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)))
                return@runOnUiThread
            }
            var finished = false
            var timeout: Runnable? = null
            fun finish(result: Result<Cipher>) {
                if (finished) return
                finished = true
                timeout?.let { handler.removeCallbacks(it) }
                releaseProtection()
                complete(result)
            }
            finishOnUi = ::finish
            timeout = Runnable {
                cancellation.cancel()
                finish(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
            }.also { handler.postDelayed(it, 60000) }
            try {
                val builder = BiometricPrompt.Builder(activity)
                    .setTitle(language.text("Odomknúť účet", "Unlock account", "Розблокувати обліковий запис"))
                    .setSubtitle(language.text("Uložená relácia Ubian", "Saved Ubian session", "Збережений сеанс Ubian"))
                    .setNegativeButton(language.text("Zrušiť", "Cancel", "Скасувати"), activity.mainExecutor) { _, _ ->
                        finish(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
                    }
                if (Build.VERSION.SDK_INT >= 30) builder.setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                builder.build().authenticate(BiometricPrompt.CryptoObject(cipher), cancellation, activity.mainExecutor,
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                            val authorized = result.cryptoObject?.cipher
                            if (authorized === cipher) finish(Result.success(cipher))
                            else finish(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
                        }
                        override fun onAuthenticationError(code: Int, message: CharSequence) {
                            val failure = if (code == BiometricPrompt.BIOMETRIC_ERROR_CANCELED ||
                                code == BiometricPrompt.BIOMETRIC_ERROR_USER_CANCELED)
                                VaultFailure.AUTHENTICATION_CANCELLED else VaultFailure.AUTHENTICATION_UNAVAILABLE
                            finish(Result.failure(VaultException(failure)))
                        }
                    })
            } catch (_: Exception) { finish(Result.failure(VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE))) }
        }
        return {
            canceled.set(true)
            cancellation.cancel()
            activity.runOnUiThread {
                finishOnUi?.invoke(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
            }
        }
    }
}
