package sk.cestou.platform

import android.os.Build
import android.security.keystore.KeyInfo
import android.security.keystore.UserNotAuthenticatedException
import android.system.Os
import android.util.AtomicFile
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.KeyStore
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

/** Device tests are synthetic-only. JCE record/file checks do not establish
 * authentication-bound Keystore round trips; interactive cases are documented
 * separately in docs/vault-device-checks.md. */
@RunWith(AndroidJUnit4::class)
class AccountVaultTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext

    private fun freshName() = "account-preparation-test-${UUID.randomUUID()}.bin"
    private fun freshAlias() = "cestou.account-preparation.test.${UUID.randomUUID()}"
    private fun jceKey(): SecretKey = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
    private fun encrypt(key: SecretKey): Cipher = Cipher.getInstance("AES/GCM/NoPadding").apply {
        init(Cipher.ENCRYPT_MODE, key)
    }
    private fun decrypt(key: SecretKey, record: ByteArray): Cipher = Cipher.getInstance("AES/GCM/NoPadding").apply {
        init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, VaultRecord.nonce(record)))
    }
    private fun expectFailure(failure: VaultFailure, operation: () -> Unit) {
        try { operation(); fail("Synthetic vault operation unexpectedly succeeded") }
        catch (error: VaultException) { assertEquals(failure, error.failure) }
    }

    @Test fun recordCodecRejectsTamperUnknownVersionAndWrongPurpose() {
        val key = jceKey()
        val synthetic = "synthetic record instrumentation".toByteArray(Charsets.UTF_8)
        try {
            val record = VaultRecord.seal(encrypt(key), synthetic)
            assertTrue(VaultRecord.verify(decrypt(key, record), record, synthetic))
            assertFalse(record.contentEquals(synthetic))
            val tampered = record.clone().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
            expectFailure(VaultFailure.TAMPERED) { VaultRecord.verify(decrypt(key, tampered), tampered, synthetic) }
            val wrongPurpose = record.clone().also { it[8] = 2 }
            expectFailure(VaultFailure.INVALID_RECORD) { VaultRecord.nonce(wrongPurpose) }
            val futureVersion = record.clone().also { it[7] = 2 }
            expectFailure(VaultFailure.UNSUPPORTED_VERSION) { VaultRecord.nonce(futureVersion) }
            val nonces = (0 until 32).map {
                VaultRecord.nonce(VaultRecord.seal(encrypt(key), synthetic)).toList()
            }.toSet()
            assertEquals(32, nonces.size)
        } finally { synthetic.fill(0) }
    }

    @Test fun atomicReplacementRollbackInterruptedWriteAndDeletion() {
        val name = freshName()
        val base = File(context.noBackupFilesDir, name)
        val files = AndroidVaultFiles(context, name)
        val synthetic = "synthetic atomic file instrumentation".toByteArray(Charsets.UTF_8)
        try {
            val record = VaultRecord.seal(encrypt(jceKey()), synthetic)
            files.write(record)
            assertArrayEquals(record, files.read())
            assertEquals(384, Os.stat(base.path).st_mode and 511) // 0600
            val platformFile = AtomicFile(base)
            val failed = platformFile.startWrite()
            failed.write(byteArrayOf(0, 1, 2))
            platformFile.failWrite(failed)
            assertArrayEquals(record, files.read())
            // Simulate process loss after data reaches a pending replacement,
            // without calling finishWrite/failWrite. A fresh reader must retain
            // the committed record on both legacy .bak and newer .new formats.
            platformFile.startWrite().use { pending ->
                pending.write(byteArrayOf(3, 4, 5))
                pending.fd.sync()
            }
            assertArrayEquals(record, AndroidVaultFiles(context, name).read())
            files.delete()
            assertNull(files.read())
            assertFalse(base.exists())
            assertFalse(File(base.path + ".bak").exists())
            assertFalse(File(base.path + ".new").exists())
        } finally { synthetic.fill(0); files.delete() }
    }

    @Test fun boundedFileReadsRejectCorruptAndOversizedRecords() {
        val name = freshName()
        val base = File(context.noBackupFilesDir, name)
        val files = AndroidVaultFiles(context, name)
        try {
            for (size in listOf(0, 36, VaultRecord.MAX_RECORD + 1)) {
                base.writeBytes(ByteArray(size))
                expectFailure(VaultFailure.INVALID_RECORD) { files.read() }
            }
        } finally { files.delete() }
    }

    @Test fun missingKeystoreKeyIsNotRecreatedForDecryption() {
        val alias = freshAlias()
        val keys = AndroidVaultKeys(context, alias)
        try {
            assertFalse(keys.exists())
            expectFailure(VaultFailure.KEY_MISSING) { keys.decryptCipher(ByteArray(12)) }
            assertFalse(keys.exists())
        } finally { keys.delete() }
    }

    @Test fun authenticationBoundKeyRefusesUnauthenticatedEncryption() {
        val alias = freshAlias()
        val keys = AndroidVaultKeys(context, alias)
        // API 26-28, no secure lock, no enrollment, or unavailable biometrics
        // skip this positive-capability case. The skip is not a passed auth test.
        assumeTrue("Requires a secure device with supported enrolled biometrics", keys.authenticationAvailable())
        val synthetic = "synthetic unauthorized key-use test".toByteArray(Charsets.UTF_8)
        try {
            keys.create()
            val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            val key = store.getKey(alias, null) as SecretKey
            assertNull(key.encoded)
            val info = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore")
                .getKeySpec(key, KeyInfo::class.java) as KeyInfo
            assertEquals(256, info.keySize)
            assertTrue(info.isUserAuthenticationRequired)
            @Suppress("DEPRECATION")
            assertTrue(info.userAuthenticationValidityDurationSeconds <= 0)
            try {
                // A successful device unlock is insufficient for a per-use key.
                // No BiometricPrompt/CryptoObject authorizes this operation.
                keys.encryptCipher().doFinal(synthetic)
                fail("Authentication-bound encryption succeeded without CryptoObject authorization")
            } catch (error: Exception) {
                val typed = error as? VaultException
                val chain = generateSequence<Throwable>(error) { it.cause }.take(16)
                assertTrue("Expected an authentication-required failure", typed?.failure == VaultFailure.AUTHENTICATION_UNAVAILABLE ||
                    chain.any { cause -> cause is UserNotAuthenticatedException ||
                        (Build.VERSION.SDK_INT >= 33 && cause is android.security.KeyStoreException && cause.requiresUserAuthentication()) })
            }
            keys.delete()
            expectFailure(VaultFailure.KEY_MISSING) { keys.encryptCipher() }
        } finally { synthetic.fill(0); keys.delete() }
    }
}
