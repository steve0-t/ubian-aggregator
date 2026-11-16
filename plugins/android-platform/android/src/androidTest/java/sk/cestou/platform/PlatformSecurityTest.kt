package sk.cestou.platform

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.UUID
import javax.crypto.AEADBadTagException

@RunWith(AndroidJUnit4::class)
class PlatformSecurityTest {
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext

    @Test fun sharedUrlCorpus() {
        val policy = RequestPolicy.parse(context.assets.open("network-policy.json").bufferedReader().use { it.readText() })
        val tests = JSONArray(InstrumentationRegistry.getInstrumentation().context.assets.open("url-policy.json").bufferedReader().use { it.readText() })
        for (index in 0 until tests.length()) {
            val case = tests.getJSONObject(index)
            val raw = case.getString("url")
            assertEquals(raw, case.getString("navigation"), policy.navigation(raw).name.lowercase())
            assertEquals(raw, case.getBoolean("resource"), policy.allowResource(raw))
        }
    }

    @Test fun pathAndSubdomainRules() {
        val policy = RequestPolicy(1, listOf("www.ubian.sk"), listOf(
            OptionalRule("www.ubian.sk", false, "/optional/", "analytics", "synthetic test"),
            OptionalRule("metrics.example.test", true, null, "analytics", "synthetic test"),
        ))
        assertFalse(policy.allowResource("https://www.ubian.sk/optional/collect"))
        assertFalse(policy.allowResource("https://a.metrics.example.test/x"))
        assertFalse(policy.allowResource("https://www.ubian.sk/public/../optional/collect"))
        assertFalse(policy.allowResource("https://www.ubian.sk/public/%2e%2e/optional/collect"))
        assertTrue(policy.allowResource("https://www.ubian.sk/login"))
        assertTrue(policy.allowResource("https://metrics.example.test.attacker.test/x"))
        assertTrue(policy.allowResource("https://notmetrics.example.test/x"))
    }

    @Test fun keystoreRoundTripTamperNoncesAndMissingKey() {
        val report = KeystoreProbe.run(context)
        assertTrue(report.roundTrip)
        assertTrue(report.tamperRejected)
        assertTrue(report.uniqueNonces)
        assertTrue(report.missingKeyRejected)
    }

    @Test fun unknownVersionInterruptedWriteAndTagFailure() {
        val vault = ProbeVault(context, "cestou.probe.test.${UUID.randomUUID()}")
        val abandoned = File(context.noBackupFilesDir, "cestou-keystore-probe.bin.new")
        val sentinel = "public instrumentation test".toByteArray()
        try {
            vault.createKey()
            val record = vault.seal(sentinel)
            vault.write(record)
            // AtomicFile must retain the committed record after an interrupted replacement.
            abandoned.writeBytes(byteArrayOf(0, 1, 2))
            assertArrayEquals(sentinel, vault.read())
            assertFalse(File(context.noBackupFilesDir, "cestou-keystore-probe.bin").readBytes().contentEquals(sentinel))
            val badVersion = record.clone().also { it[3] = 2 }
            try { vault.open(badVersion); fail("Unknown version accepted") } catch (_: IllegalArgumentException) { }
            val badTag = record.clone().also { it[it.lastIndex] = (it.last().toInt() xor 1).toByte() }
            try { vault.open(badTag); fail("Invalid tag accepted") } catch (_: AEADBadTagException) { }
            val nonces = (0 until 32).map { vault.seal(sentinel).copyOfRange(4, 16).toList() }.toSet()
            assertEquals(32, nonces.size)
        } finally { sentinel.fill(0); vault.delete(); abandoned.delete() }
    }
}
