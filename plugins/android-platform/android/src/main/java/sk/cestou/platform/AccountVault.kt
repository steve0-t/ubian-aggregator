package sk.cestou.platform

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import android.system.Os
import android.util.AtomicFile
import android.view.WindowManager
import java.io.File
import java.security.KeyStore
import java.security.UnrecoverableKeyException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Account-preparation namespace, disjoint from public probes and future real credentials. */
internal class AndroidVaultKeys(private val context: Context, private val alias: String) : VaultKeys {
    private val keys = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    @Suppress("DEPRECATION")
    override fun authenticationAvailable(): Boolean {
        if (Build.VERSION.SDK_INT < 29 || !context.getSystemService(KeyguardManager::class.java).isDeviceSecure) return false
        return when {
            Build.VERSION.SDK_INT >= 30 -> context.getSystemService(BiometricManager::class.java)
                .canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) == BiometricManager.BIOMETRIC_SUCCESS
            else -> context.getSystemService(BiometricManager::class.java)
                .canAuthenticate() == BiometricManager.BIOMETRIC_SUCCESS
        }
    }

    override fun exists(): Boolean = keys.containsAlias(alias)

    @Suppress("DEPRECATION")
    override fun create() {
        if (!authenticationAvailable()) throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
        if (exists()) return
        val parameters = KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setKeySize(256)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setRandomizedEncryptionRequired(true)
            .setUserAuthenticationRequired(true)
            .setInvalidatedByBiometricEnrollment(true)
        if (Build.VERSION.SDK_INT >= 30) parameters.setUserAuthenticationParameters(0, KeyProperties.AUTH_BIOMETRIC_STRONG)
        else parameters.setUserAuthenticationValidityDurationSeconds(-1)
        KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(parameters.build())
            generateKey()
        }
    }

    private fun cipher(mode: Int, nonce: ByteArray? = null): Cipher = try {
        val key = keys.getKey(alias, null) as? SecretKey ?: throw VaultException(VaultFailure.KEY_MISSING)
        Cipher.getInstance("AES/GCM/NoPadding").apply {
            if (nonce == null) init(mode, key) else init(mode, key, GCMParameterSpec(128, nonce))
        }
    } catch (_: KeyPermanentlyInvalidatedException) { throw VaultException(VaultFailure.KEY_INVALIDATED) }
      catch (_: UnrecoverableKeyException) { throw VaultException(VaultFailure.KEY_INVALIDATED) }
      catch (_: UserNotAuthenticatedException) { throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE) }

    override fun encryptCipher(): Cipher = cipher(Cipher.ENCRYPT_MODE)
    override fun decryptCipher(nonce: ByteArray): Cipher = cipher(Cipher.DECRYPT_MODE, nonce)
    override fun delete() { keys.deleteEntry(alias) }
    override fun failure(error: Throwable): VaultException {
        var current: Throwable? = error
        repeat(8) {
            when (val cause = current) {
                is VaultException -> return cause
                is KeyPermanentlyInvalidatedException, is UnrecoverableKeyException -> return VaultException(VaultFailure.KEY_INVALIDATED)
                is UserNotAuthenticatedException -> return VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
            }
            current = current?.cause
        }
        return VaultException(VaultFailure.IO_FAILURE)
    }
}

internal class AndroidVaultFiles(context: Context, name: String) : VaultFiles {
    private val file = AtomicFile(File(context.noBackupFilesDir, name))

    override fun read(): ByteArray? = try {
        file.openRead().use { input ->
            val result = input.readBytesAtMost(VaultRecord.MAX_RECORD)
            VaultRecord.nonce(result)
            result
        }
    } catch (_: java.io.FileNotFoundException) {
        if (file.baseFile.exists() || File(file.baseFile.path + ".bak").exists() || File(file.baseFile.path + ".new").exists()) {
            throw VaultException(VaultFailure.IO_FAILURE)
        }
        null
    }

    override fun write(record: ByteArray) {
        VaultRecord.nonce(record)
        val stream = file.startWrite()
        try {
            Os.fchmod(stream.fd, 384) // 0600
            stream.write(record)
            stream.fd.sync()
            file.finishWrite(stream)
            // AtomicFile's platform API returns void. Verify replacement rather
            // than claiming a swallowed rename/sync failure committed the write.
            if (!record.contentEquals(read())) throw VaultException(VaultFailure.IO_FAILURE)
        } catch (error: Exception) { file.failWrite(stream); throw VaultException(VaultFailure.IO_FAILURE) }
    }

    override fun delete() {
        file.delete()
        if (file.baseFile.exists() || File(file.baseFile.path + ".bak").exists() || File(file.baseFile.path + ".new").exists()) {
            throw VaultException(VaultFailure.IO_FAILURE)
        }
    }

    private fun java.io.InputStream.readBytesAtMost(limit: Int): ByteArray {
        val bytes = readNBytesCompat(limit + 1)
        if (bytes.size > limit) throw VaultException(VaultFailure.INVALID_RECORD)
        return bytes
    }

    private fun java.io.InputStream.readNBytesCompat(limit: Int): ByteArray {
        val output = java.io.ByteArrayOutputStream()
        val chunk = ByteArray(1024)
        while (output.size() < limit) {
            val count = read(chunk, 0, minOf(chunk.size, limit - output.size()))
            if (count < 0) break
            output.write(chunk, 0, count)
        }
        return output.toByteArray()
    }
}

/** Per-operation CryptoObject authorization; no time-window/plaintext fallback.
 * Device credential fallback for this key type needs separate verification.
 * API 26–28 decline secure persistence, while manual provider login stays usable. */
internal class AndroidVaultAuthorization(private val activity: Activity, private val language: AppLanguage) : VaultAuthorization {
    override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
        if (Build.VERSION.SDK_INT < 29) throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
        val cancellation = CancellationSignal()
        activity.runOnUiThread {
            if (activity.isFinishing || activity.isDestroyed || cancellation.isCanceled) {
                complete(Result.failure(VaultException(VaultFailure.AUTHENTICATION_CANCELLED)))
                return@runOnUiThread
            }
            val alreadySecure = activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE != 0
            activity.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            var finished = false
            fun finish(result: Result<Cipher>) {
                if (finished) return
                finished = true
                if (!alreadySecure) activity.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                complete(result)
            }
            try {
                val builder = BiometricPrompt.Builder(activity)
                    .setTitle(language.text("Test úložiska", "Storage test", "Перевірка сховища"))
                    .setSubtitle(language.text("Iba syntetické údaje", "Synthetic data only", "Лише синтетичні дані"))
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
                        // A failed scan leaves the prompt open. The OS bounds attempts.
                    })
            } catch (_: Exception) { finish(Result.failure(VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE))) }
        }
        return { cancellation.cancel() }
    }
}

internal object AccountVault {
    private var instance: SyntheticVault? = null
    @Synchronized fun get(activity: Activity): SyntheticVault = instance ?: SyntheticVault(
        AndroidVaultKeys(activity.applicationContext, "cestou.account-preparation.synthetic.v1"),
        AndroidVaultFiles(activity.applicationContext, "account-preparation-synthetic.v1.bin"),
        // No frontend operation invokes authorization yet. Instrumentation owns
        // any interactive synthetic run and its chosen language/lifecycle.
        object : VaultAuthorization {
            override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit =
                throw VaultException(VaultFailure.AUTHENTICATION_UNAVAILABLE)
        },
    ).also { instance = it }
}
