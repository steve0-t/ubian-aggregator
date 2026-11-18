package sk.cestou.platform

import android.system.Os
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import java.util.concurrent.Executor
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Synthetic snapshots only. These tests exercise the real session codec and
 * coordinator, but substitute authorization/keys; they do not prove a device's
 * biometric CryptoObject or authentication-bound Keystore behavior. */
@RunWith(AndroidJUnit4::class)
class RealSessionVaultTest {
    private val direct = Executor { it.run() }
    private val immediate = object : VaultAuthorization {
        override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
            complete(Result.success(cipher))
            return {}
        }
    }
    private class Keys : VaultKeys {
        var key: SecretKey? = null
        var created = 0
        var failDelete = false
        override fun authenticationAvailable() = true
        override fun exists() = key != null
        override fun create() {
            created++
            key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        }
        override fun encryptCipher() = Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.ENCRYPT_MODE, key ?: throw VaultException(VaultFailure.KEY_MISSING))
        }
        override fun decryptCipher(nonce: ByteArray) = Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.DECRYPT_MODE, key ?: throw VaultException(VaultFailure.KEY_MISSING), GCMParameterSpec(128, nonce))
        }
        override fun delete() {
            if (failDelete) throw VaultException(VaultFailure.IO_FAILURE)
            key = null
        }
    }
    private class Files : VaultFiles {
        var record: ByteArray? = null
        var deletions = 0
        override fun read() = record?.clone()
        override fun write(record: ByteArray) { this.record = record.clone() }
        override fun delete() { deletions++; record = null }
    }
    private fun vault(keys: Keys, files: VaultFiles, authorization: VaultAuthorization = immediate) =
        RealSessionVault(keys, files, { authorization }, direct)

    @Test fun realSessionRoundTripFreshNonceAndTampering() {
        val keys = Keys()
        val files = Files()
        val vault = vault(keys, files)
        val expected = "{\"synthetic\":\"session fixture\"}".toByteArray()
        val input = expected.clone()
        var saved = false
        vault.store(input, AppLanguage.EN) { saved = it.isSuccess }
        assertTrue(saved)
        assertTrue(input.all { it == 0.toByte() })
        val first = files.record!!.clone()
        assertFalse(first.contentEquals(expected))
        var restored: ByteArray? = null
        var delivered: ByteArray? = null
        vault.restore(AppLanguage.EN) {
            delivered = it.getOrThrow()
            restored = it.getOrThrow().clone()
        }
        assertArrayEquals(expected, restored)
        assertTrue(delivered!!.all { it == 0.toByte() })
        vault.store(expected.clone(), AppLanguage.SK) { assertTrue(it.isSuccess) }
        assertFalse(SessionRecord.nonce(first).contentEquals(SessionRecord.nonce(files.record!!)))
        files.record!![files.record!!.lastIndex] = (files.record!!.last().toInt() xor 1).toByte()
        vault.restore(AppLanguage.UK) {
            assertEquals(VaultFailure.TAMPERED, (it.exceptionOrNull() as VaultException).failure)
        }
    }

    @Test fun cancelAndDeleteFenceLateAuthorizationAndWipePendingInput() {
        val keys = Keys()
        val files = Files()
        var approvedCipher: Cipher? = null
        var later: ((Result<Cipher>) -> Unit)? = null
        val delayed = object : VaultAuthorization {
            override fun authorize(cipher: Cipher, complete: (Result<Cipher>) -> Unit): () -> Unit {
                approvedCipher = cipher
                later = complete
                return {}
            }
        }
        val vault = vault(keys, files, delayed)
        val input = "{\"synthetic\":true}".toByteArray()
        var completions = 0
        vault.store(input, AppLanguage.SK) {
            completions++
            assertEquals(VaultFailure.AUTHENTICATION_CANCELLED, (it.exceptionOrNull() as VaultException).failure)
        }
        assertTrue(vault.delete().isSuccess)
        assertTrue(input.all { it == 0.toByte() })
        later!!(Result.success(approvedCipher!!))
        assertNull(files.record)
        assertFalse(keys.exists())
        assertEquals(1, completions)
    }

    @Test fun missingKeyDoesNotReassignStoredSessionAndPartialDeletionStillDeletesFile() {
        val keys = Keys().apply { create() }
        val files = Files()
        val vault = vault(keys, files)
        vault.store("{\"synthetic\":true}".toByteArray(), AppLanguage.EN) { assertTrue(it.isSuccess) }
        keys.key = null
        vault.restore(AppLanguage.EN) {
            assertEquals(VaultFailure.KEY_MISSING, (it.exceptionOrNull() as VaultException).failure)
        }
        assertEquals(1, keys.created)
        assertEquals("key_missing", vault.status().availability)
        assertTrue(vault.status().recordPresent)
        keys.failDelete = true
        assertTrue(vault.delete().isFailure)
        assertNull(files.record)
        assertEquals(1, files.deletions)
    }

    @Test fun codecRejectsUnknownVersionWrongPurposeAndOversize() {
        val keys = Keys().apply { create() }
        val record = SessionRecord.seal(keys.encryptCipher(), byteArrayOf(1))
        val future = record.clone().also { it[7] = 2 }
        try { SessionRecord.nonce(future); fail("Future session version was accepted") }
        catch (error: VaultException) { assertEquals(VaultFailure.UNSUPPORTED_VERSION, error.failure) }
        val purpose = record.clone().also { it[8] = 1 }
        try { SessionRecord.nonce(purpose); fail("Synthetic account purpose was accepted") }
        catch (error: VaultException) { assertEquals(VaultFailure.INVALID_RECORD, error.failure) }
        try { SessionRecord.seal(keys.encryptCipher(), ByteArray(SessionRecord.MAX_PLAINTEXT + 1)); fail("Oversize snapshot was accepted") }
        catch (error: VaultException) { assertEquals(VaultFailure.INVALID_RECORD, error.failure) }
    }

    @Test fun atomicSessionFileUsesPrivateModeAndIsDeletableWhenCorrupt() {
        val directory = InstrumentationRegistry.getInstrumentation().targetContext.noBackupFilesDir
        val name = "native-session-instrumentation-${UUID.randomUUID()}.bin"
        val base = File(directory, name)
        val files = SessionFiles(directory, name)
        val keys = Keys().apply { create() }
        try {
            val record = SessionRecord.seal(keys.encryptCipher(), "{\"synthetic\":true}".toByteArray())
            files.write(record)
            assertArrayEquals(record, files.read())
            assertEquals(384, Os.stat(base.path).st_mode and 511)
            base.writeBytes(ByteArray(20))
            val status = vault(keys, files).status()
            assertEquals("invalid_record", status.availability)
            assertTrue(status.recordPresent)
            files.delete()
            assertNull(files.read())
        } finally { files.delete() }
    }
}
