import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "sk.cestou.platform"
    compileSdk = 37
    defaultConfig {
        minSdk = 26
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    // The DSL block avoids the legacy sourceSets getter's incompatible
    // AndroidLibrarySourceSet cast on AGP 9.3.
    sourceSets {
        getByName("main") { assets.srcDir("../../../policy") }
        getByName("androidTest") { assets.srcDir("../../../tests/fixtures") }
    }
}

kotlin {
    compilerOptions { jvmTarget = JvmTarget.JVM_1_8 }
}

dependencies {
    implementation(project(":tauri-android"))
    // Tauri's lifecycle hooks name this type, but its implementation dependency
    // is not exported onto the plugin's compile classpath.
    compileOnly("androidx.appcompat:appcompat:1.7.1")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}
