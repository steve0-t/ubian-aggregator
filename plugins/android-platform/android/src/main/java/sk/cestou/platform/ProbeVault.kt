package sk.cestou.platform

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.system.Os
import android.util.AtomicFile
import java.io.File
import java.nio.ByteBuffer
import java.security.KeyStore
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import java.util.UUID

/** Milestone-1 test storage only. No credential save/read command is exported. */
internal class ProbeVault(context: Context, private val alias: String) {
    private val file = AtomicFile(File(context.noBackupFilesDir, "cestou-keystore-probe.bin"))
    private val keys = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private val aad = "cestou|1|keystore-probe".toByteArray(Charsets.UTF_8)

    fun createKey() {
        require(!keys.containsAlias(alias))
        KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                // Probe data is public. Credential reuse requires an additional,
                // tested authentication-bound key design in milestone 3.
                .setUserAuthenticationRequired(false)
                .build())
            generateKey()
        }
    }

    private fun key(): SecretKey = keys.getKey(alias, null) as? SecretKey ?: error("missing_key")

    @Suppress("DEPRECATION")
    fun hardwareBacked(): Boolean =
        SecretKeyFactory.getInstance(key().algorithm, "AndroidKeyStore")
            .getKeySpec(key(), KeyInfo::class.java).let { it as KeyInfo }.isInsideSecureHardware

    fun seal(plaintext: ByteArray): ByteArray {
        require(plaintext.size <= 65536)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(aad)
        require(cipher.iv.size == 12)
        val ciphertext = cipher.doFinal(plaintext)
        return ByteBuffer.allocate(4 + 12 + ciphertext.size).putInt(1).put(cipher.iv).put(ciphertext).array()
    }

    fun open(record: ByteArray): ByteArray {
        require(record.size in 32..65568)
        val buffer = ByteBuffer.wrap(record)
        require(buffer.int == 1) { "unsupported_version" }
        val nonce = ByteArray(12).also { buffer.get(it) }
        val ciphertext = ByteArray(buffer.remaining()).also { buffer.get(it) }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, nonce))
        cipher.updateAAD(aad)
        return cipher.doFinal(ciphertext)
    }

    fun write(record: ByteArray) {
        require(record.size in 32..65568)
        val stream = file.startWrite()
        try {
            Os.fchmod(stream.fd, 384) // 0600
            stream.write(record)
            file.finishWrite(stream)
        } catch (error: Exception) { file.failWrite(stream); throw error }
    }

    fun read(): ByteArray {
        return file.openRead().use { input ->
            val data = input.readBytesLimited(65568)
            open(data)
        }
    }

    fun deleteKey() { keys.deleteEntry(alias) }
    fun delete() { file.delete(); deleteKey() }

    private fun java.io.InputStream.readBytesLimited(limit: Int): ByteArray {
        val out = java.io.ByteArrayOutputStream()
        val chunk = ByteArray(4096)
        while (true) {
            val count = read(chunk)
            if (count == -1) break
            require(out.size() + count <= limit) { "invalid_record" }
            out.write(chunk, 0, count)
        }
        return out.toByteArray()
    }
}

internal data class ProbeReport(
    val roundTrip: Boolean,
    val tamperRejected: Boolean,
    val uniqueNonces: Boolean,
    val missingKeyRejected: Boolean,
    val hardwareBacked: Boolean,
)

internal object KeystoreProbe {
    @Synchronized
    fun run(context: Context): ProbeReport {
        // Recover only public probe artifacts left by process death. Credential
        // keys are deliberately outside this namespace and this implementation.
        val staleKeys = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        staleKeys.aliases().toList().filter { it.startsWith("cestou.probe.") }.forEach { staleKeys.deleteEntry(it) }
        val vault = ProbeVault(context, "cestou.probe.${UUID.randomUUID()}")
        val sentinel = "Cestou public encryption probe".toByteArray(Charsets.UTF_8)
        try {
            vault.createKey()
            val hardware = vault.hardwareBacked()
            val first = vault.seal(sentinel)
            val second = vault.seal(sentinel)
            vault.write(first)
            val restored = vault.read()
            val roundTrip = restored.contentEquals(sentinel)
            restored.fill(0)
            val tampered = first.clone().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
            val rejected = try { vault.open(tampered).fill(0); false } catch (_: AEADBadTagException) { true }
            vault.deleteKey()
            val missing = try { vault.open(first).fill(0); false } catch (_: IllegalStateException) { true }
            return ProbeReport(roundTrip, rejected, !first.copyOfRange(4, 16).contentEquals(second.copyOfRange(4, 16)), missing, hardware)
        } finally { sentinel.fill(0); vault.delete() }
    }
}
